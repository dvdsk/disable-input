use std::io::{Read, BufReader, BufRead, self};
use std::sync::mpsc::{self, TryRecvError};
use std::thread::{self, JoinHandle};

pub struct BackgroundLineReader {
    _handle: JoinHandle<()>,
    lines: mpsc::Receiver<Result<String, io::Error>>,
}

impl BackgroundLineReader {
    pub fn new(reader: impl Read + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let reader = BufReader::new(reader);
            for line in reader.lines() {
                let err_happend = line.is_err();
                tx.send(line).unwrap();
                if err_happend {
                    return;
                }
            }
        });
        Self {
            _handle: handle,
            lines: rx,
        }
    }

    /// get any lines
    pub fn lines(&mut self) -> Result<Vec<String>, io::Error> {
        let mut lines = Vec::new();
        loop {
            match self.lines.try_recv() {
                Ok(Ok(line)) => lines.push(line),
                Ok(Err(e)) => return Err(e),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return Ok(lines),
            }
        }
    }
}
