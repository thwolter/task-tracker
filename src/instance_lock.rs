//! Process-wide startup lock for Tempo.
//!
//! Holding [`InstanceLock`] keeps the current process registered as Tempo's
//! only running instance. The platform-specific handle is released on drop.

use crate::{error::Result, persistence::SqliteStore};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use fs2::FileExt;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use single_instance::SingleInstance;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::fs::{File, OpenOptions};

/// An owned guard proving that this process acquired Tempo's startup lock.
pub(crate) struct InstanceLock {
    _platform_lock: PlatformLock,
}

enum PlatformLock {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    File { _file: File },
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    System { _instance: SingleInstance },
}

/// Acquires Tempo's single-instance lock.
///
/// Returns `Ok(None)` when another Tempo process already holds the lock.
pub(crate) fn acquire() -> Result<Option<InstanceLock>> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        let data_dir = SqliteStore::default_data_dir();
        std::fs::create_dir_all(&data_dir)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(data_dir.join("tempo.lock"))?;

        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(InstanceLock {
                _platform_lock: PlatformLock::File { _file: file },
            })),
            Err(error) if error.kind() == fs2::lock_contended_error().kind() => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let instance = SingleInstance::new("com.thomas.tempo")?;
        Ok(instance.is_single().then_some(InstanceLock {
            _platform_lock: PlatformLock::System {
                _instance: instance,
            },
        }))
    }
}
