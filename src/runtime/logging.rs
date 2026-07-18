use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug)]
pub struct EventLogger {
    file: File,
    echo_json: bool,
}

#[derive(Serialize)]
struct Event<'a, T> {
    timestamp_unix_ms: u64,
    event: &'a str,
    data: T,
}

impl EventLogger {
    pub fn create(path: &Path, echo_json: bool) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .with_context(|| format!("could not create event log {}", path.display()))?;
        Ok(Self { file, echo_json })
    }

    pub fn write<T: Serialize>(&mut self, kind: &str, data: T) -> Result<()> {
        let event = Event {
            timestamp_unix_ms: unix_milliseconds(),
            event: kind,
            data,
        };
        serde_json::to_writer(&mut self.file, &event)?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;

        if self.echo_json {
            let stdout = io::stdout();
            let mut output = stdout.lock();
            serde_json::to_writer(&mut output, &event)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
        Ok(())
    }
}

pub fn unix_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
