use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::Duration;

mod background_reader;
use background_reader::BackgroundLineReader;

#[derive(Debug)]
pub enum CommandError {
    Io(std::io::Error),
    Failed { stderr: String },
}

pub struct LockedDevice {
    process: Child,
    check_thread: JoinHandle<()>,
}

impl LockedDevice {
    pub fn unlock(self) {
        core::mem::drop(self);
    }
}

impl Drop for LockedDevice {
    fn drop(&mut self) {
        self.process.kill().unwrap();
    }
}

impl Device {
    #[must_use]
    pub fn lock(self) -> Result<LockedDevice, CommandError> {
        let mut process = Command::new("evtest")
            .arg("--grab")
            .arg(&self.event_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(CommandError::Io)?;

        let stderr = process.stderr.take().unwrap();

        let check_thread = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let mut error = Vec::new();
            for line in reader.lines().take(5) {
                let Ok(line) = line else {
                panic!("Could not grab device\n\tstderr: {error:?}");
            };
                error.push(line);
            }
            if error.len() < 5 {
                panic!("Could not grab device\n\tstderr: {error:?}");
            }
        });

        Ok(LockedDevice {
            process,
            check_thread,
        })
    }
}

#[derive(Debug)]
pub struct Device {
    pub event_path: String,
    pub name: String,
}

pub fn list() -> Result<Vec<Device>, CommandError> {
    let mut handle = Command::new("evtest")
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CommandError::Io)?;
    let mut reader = BackgroundLineReader::new(handle.stderr.take().unwrap());
    println!("discovering input devices");
    thread::sleep(Duration::from_secs(5));
    Ok(reader
        .lines()
        .map_err(CommandError::Io)?
        .into_iter()
        .filter(|s| s.starts_with("/dev/input/event"))
        .map(|s| {
            let (event_path, name) = s.split_once(":").unwrap();
            let event_path = event_path.trim().to_string();
            let name = name.trim().to_string();
            Device { event_path, name }
        })
        .collect())
}
