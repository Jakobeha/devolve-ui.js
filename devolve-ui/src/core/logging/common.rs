use std::cell::RefCell;
use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::io::Write;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};
use chrono::{DateTime, Utc};
use serde::Serialize;

//! Constructor passed to both loggers, so they can share the same start-time and file.
#[derive(Debug, Clone)]
pub struct LogStart {
    monotonic_time: Instant,
    user_time: DateTime<Utc>,
    dir: PathBuf,
    shared_file: Rc<RefCell<File>>
}

#[derive(Serialize, Deserialize)]
pub(super) struct LogEntry<T> {
    pub time: Duration,
    pub data: T
}

/// Common datatypes in both loggers. *Not* common data, common constructor data is `LogStart`
pub(super) struct GenericLogger<T> {
    monotonic_start_time: Instant,
    shared_file: Rc<RefCell<File>>,
    specific_file: File,
    phantom: PhantomData<T>
}

#[derive(Debug)]
pub enum LogError {
    IO(io::Error),
    Serde(serde_json::Error)
}

type LogResult = Result<(), LogError>;

impl From<io::Error> for LogError {
    fn from(err: io::Error) -> Self {
        Self::IO(err)
    }
}

impl From<serde_json::Error> for LogError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(err)
    }
}

impl LogStart {
    pub fn new(dir: &Path) -> LogStart {
        let dir = dir.to_path_buf();
        let monotonic_time = Instant::now();
        let user_time: DateTime<Utc> = Utc::now();
        let shared_file = Rc::new(RefCell::new(Self::open_log(&dir, user_time, "all")));
        LogStart { monotonic_time, user_time, dir, shared_file }
    }

    pub fn create_log(dir: &Path, user_time: &DateTime<Utc>, name: &str) -> io::Result<File> {
        let path = Self::log_path(dir, user_time, name);
        File::options().write(true).create_new(true).open(path)
    }

    fn log_path(dir: &Path, user_time: &DateTime<Utc>, name: &str) -> PathBuf {
        PathBuf::from(format!("{}/{}-{}.json", dir.display(), user_time.format("%FT%T"), name))
    }
}

impl <T: Serialize + Debug> GenericLogger<T> {
    pub fn new(args: &LogStart, name: &str) -> io::Result<Self> {
        let specific_file = LogStart::create_log(&args.dir, &args.user_time, name)?;
        Ok(GenericLogger {
            monotonic_start_time: args.monotonic_time,
            shared_file: args.shared_file.clone(),
            specific_file,
            phantom: PhantomData
        })
    }

    fn do_log(&mut self, data: T) -> LogResult {
        let entry = LogEntry {
            time: self.monotonic_start_time.elapsed(),
            data
        };
        let serial = serde_json::to_vec(&entry)?;

        {
            let mut shared_file = self.shared_file.borrow_mut();
            shared_file.write_all(&serial)?;
            shared_file.write_all(b"\n")?;
        }
        self.specific_file.write_all(&serial)?;
        self.specific_file.write_all(b"\n")?;

        Ok(())
    }

    pub fn log(&mut self, data: T) {
        let result = self.do_log(data);
        if let Err(err) = result {
            eprintln!("Error logging: data={:?}, error={:?}", data, err);
        }
    }
}