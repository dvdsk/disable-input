use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

mod background_reader;
use background_reader::BackgroundLineReader;

#[derive(Debug)]
pub enum CommandError {
    Io(std::io::Error),
    Failed { stderr: String },
}

pub struct LockedDevice {
    process: Arc<Mutex<Child>>,
    stopping: Arc<AtomicBool>,
    maintain_lock: JoinHandle<()>,
}

impl LockedDevice {
    pub fn unlock(self) {
        core::mem::drop(self);
    }
}

impl Drop for LockedDevice {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        self.process.lock().unwrap().kill().unwrap();
    }
}

impl Device {
    #[must_use]
    pub fn lock(self) -> Result<LockedDevice, CommandError> {
        let Self { event_path, .. } = self;
        let (process, stderr) = lock_input(&event_path)?;
        let process = Arc::new(Mutex::new(process));
        let stopping = Arc::new(AtomicBool::new(false));

        let first_lock = Instant::now();
        let maintain_lock = {
            let process = process.clone();
            let stopping = stopping.clone();
            thread::spawn(move || {
                let mut stderr = Some(stderr);
                loop {
                    let err = wait_for_stderr_end(stderr.take().unwrap());
                    if stopping.load(Ordering::Relaxed) {
                        break;
                    }
                    if first_lock.elapsed() < Duration::from_secs(5) {
                        panic!("{err}");
                    }
                    // todo figure out startup vs keyboard in/out error
                    let (new_process, new_stderr) = lock_input(&event_path).unwrap();
                    *process.lock().unwrap() = new_process;
                    stderr = Some(new_stderr);
                }
            })
        };

        Ok(LockedDevice {
            process,
            maintain_lock,
            stopping,
        })
    }
}

fn lock_input(event_path: &str) -> Result<(Child, ChildStderr), CommandError> {
    let mut process = Command::new("evtest")
        .arg("--grab")
        .arg(event_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CommandError::Io)?;
    let stderr = process.stderr.take().unwrap();
    Ok((process, stderr))
}

fn wait_for_stderr_end(stderr: ChildStderr) -> String {
    let reader = BufReader::new(stderr);
    let mut error = Vec::new();
    for line in reader.lines().take(5) {
        error.push(line.unwrap());
    }
    error.as_slice().join("\n")
}

#[derive(Debug, Clone)]
pub struct Device {
    pub event_path: String,
    pub name: String,
}

pub fn list() -> Result<Vec<Device>, CommandError> {
    let mut handle = Command::new("evtest")
        .stdout(Stdio::null())
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
