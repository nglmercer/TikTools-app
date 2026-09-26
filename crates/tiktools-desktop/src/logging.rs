//! Release-safe host logging.
//!
//! The desktop executable is a Windows GUI subsystem binary in release mode,
//! so stdout is not a reliable place for startup diagnostics. This module
//! keeps the existing `tracing` call sites and supplies a small bounded file
//! writer instead of creating an unbounded log file. Every line is also
//! mirrored to stderr (best-effort, ignored when there is no console) so a
//! terminal launch shows plugin lifecycle and warnings live.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tiktools_core::paths::AppPaths;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter};

const MAX_LOG_BYTES: u64 = 4 * 1024 * 1024;
const MAX_ROTATED_FILES: usize = 5;

pub fn init() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let paths = AppPaths::from_environment();
    paths.ensure_directories()?;
    let log_path = paths.logs.join("tiktools.log");
    let writer = RotatingLogWriter::new(&log_path)?;
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("tiktools=info")),
        )
        .with_writer(writer)
        .try_init()
        .map_err(|error| io::Error::other(error.to_string()))?;
    Ok(log_path)
}

#[derive(Clone)]
struct RotatingLogWriter {
    state: Arc<Mutex<LogFile>>,
}

impl RotatingLogWriter {
    fn new(path: &Path) -> io::Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(LogFile::open(path)?)),
        })
    }
}

impl<'a> MakeWriter<'a> for RotatingLogWriter {
    type Writer = TeeWriter<LogGuard, io::Stderr>;

    fn make_writer(&'a self) -> Self::Writer {
        TeeWriter {
            primary: LogGuard {
                state: Arc::clone(&self.state),
            },
            secondary: io::stderr(),
        }
    }
}

/// Fans one formatted line out to the log file (authoritative) and stderr
/// (live console mirror). The secondary is strictly best-effort: a missing
/// console (Windows GUI release) or a closed pipe must never fail or
/// short-circuit the file write.
struct TeeWriter<A, B> {
    primary: A,
    secondary: B,
}

impl<A: Write, B: Write> Write for TeeWriter<A, B> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.primary.write(bytes)?;
        let _ = self.secondary.write_all(bytes);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.primary.flush()?;
        let _ = self.secondary.flush();
        Ok(())
    }
}

struct LogGuard {
    state: Arc<Mutex<LogFile>>,
}

impl Write for LogGuard {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Fail-closed on purpose: poison recovery logs via tracing, which
        // would reenter this writer; dropping one log write is safer.
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("TikTools log writer lock poisoned"))?;
        state.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        // Fail-closed like `write` above: never recover through tracing
        // from inside the log writer itself.
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("TikTools log writer lock poisoned"))?;
        state.file_mut()?.flush()
    }
}

struct LogFile {
    path: PathBuf,
    file: Option<File>,
    size: u64,
}

impl LogFile {
    fn open(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let size = fs::metadata(path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            path: path.to_owned(),
            file: Some(file),
            size,
        })
    }

    fn file_mut(&mut self) -> io::Result<&mut File> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("TikTools log file is closed"))
    }

    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.size.saturating_add(bytes.len() as u64) > MAX_LOG_BYTES {
            self.rotate()?;
        }
        let written = self.file_mut()?.write(bytes)?;
        self.size = self.size.saturating_add(written as u64);
        Ok(written)
    }

    fn rotate(&mut self) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        for index in (1..MAX_ROTATED_FILES).rev() {
            let source = rotated_path(&self.path, index);
            let destination = rotated_path(&self.path, index + 1);
            if destination.exists() {
                let _ = fs::remove_file(&destination);
            }
            if source.exists() {
                fs::rename(source, destination)?;
            }
        }
        let first = rotated_path(&self.path, 1);
        if first.exists() {
            let _ = fs::remove_file(&first);
        }
        if self.path.exists() {
            fs::rename(&self.path, &first)?;
        }
        self.file = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?,
        );
        self.size = 0;
        Ok(())
    }
}

fn rotated_path(path: &Path, index: usize) -> PathBuf {
    PathBuf::from(format!("{}.{}", path.display(), index))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn log_writer_creates_and_bounds_rotated_files() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("tiktools-logging-{suffix}"));
        let path = directory.join("tiktools.log");
        let mut log = LogFile::open(&path).unwrap();
        let bytes = vec![b'x'; MAX_LOG_BYTES as usize];
        log.write(&bytes).unwrap();
        log.write(b"next\n").unwrap();
        assert!(path.is_file());
        assert!(rotated_path(&path, 1).is_file());
        assert!(!rotated_path(&path, MAX_ROTATED_FILES + 1).exists());
        let _ = fs::remove_dir_all(directory);
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("no console"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("no console"))
        }
    }

    #[test]
    fn tee_writer_mirrors_to_both_and_tolerates_console_failure() {
        let mut mirror = TeeWriter {
            primary: Vec::new(),
            secondary: Vec::new(),
        };
        mirror.write_all(b"hello\n").unwrap();
        mirror.flush().unwrap();
        assert_eq!(mirror.primary, b"hello\n");
        assert_eq!(mirror.secondary, b"hello\n");

        // A dead console (Windows GUI release, closed pipe) must never
        // fail the authoritative file write.
        let mut no_console = TeeWriter {
            primary: Vec::new(),
            secondary: FailingWriter,
        };
        no_console.write_all(b"hello\n").unwrap();
        no_console.flush().unwrap();
        assert_eq!(no_console.primary, b"hello\n");
    }
}
