use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

pub type PersistenceResult<T> = Result<T, PersistenceError>;

#[derive(Debug)]
pub enum PersistenceError {
    Serialize(serde_json::Error),
    Decode(serde_json::Error),
    Io(std::io::Error),
    InvalidPath,
    MissingEnvironment(&'static str),
}

impl PersistenceError {
    pub fn category(&self) -> &'static str {
        match self {
            Self::Serialize(_) => "serialize",
            Self::Decode(_) => "decode",
            Self::Io(_) => "io",
            Self::InvalidPath => "invalidPath",
            Self::MissingEnvironment(_) => "missingEnvironment",
        }
    }
}

impl Display for PersistenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(error) => write!(formatter, "failed to serialize JSON: {error}"),
            Self::Decode(error) => write!(formatter, "failed to decode JSON: {error}"),
            Self::Io(error) => write!(formatter, "JSON store I/O failed: {error}"),
            Self::InvalidPath => {
                formatter.write_str("JSON store path must have a parent directory")
            }
            Self::MissingEnvironment(name) => {
                write!(
                    formatter,
                    "required Windows environment variable {name} is unavailable"
                )
            }
        }
    }
}

impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialize(error) | Self::Decode(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::InvalidPath => None,
            Self::MissingEnvironment(_) => None,
        }
    }
}

impl From<std::io::Error> for PersistenceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct AtomicJsonStore;

impl AtomicJsonStore {
    pub fn write<T>(path: &Path, value: &T) -> PersistenceResult<()>
    where
        T: Serialize + ?Sized,
    {
        let bytes = serde_json::to_vec_pretty(value).map_err(PersistenceError::Serialize)?;
        let parent = path.parent().ok_or(PersistenceError::InvalidPath)?;
        fs::create_dir_all(parent)?;

        let temporary_path =
            path.with_extension(match path.extension().and_then(|value| value.to_str()) {
                Some(extension) => format!("{extension}.tmp"),
                None => "tmp".to_owned(),
            });
        let result = (|| -> PersistenceResult<()> {
            let mut file = File::create(&temporary_path)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            replace_file(&temporary_path, path)?;
            Ok(())
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }

    pub fn read<T>(path: &Path) -> PersistenceResult<Option<T>>
    where
        T: DeserializeOwned,
    {
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path)?;
        let value = serde_json::from_slice(&bytes).map_err(PersistenceError::Decode)?;
        Ok(Some(value))
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: Both paths are owned, NUL-terminated UTF-16 buffers that remain
    // alive for the duration of this synchronous Win32 call.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
