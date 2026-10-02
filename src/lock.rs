//! File lock for storage – exclusive flock on TontooOS/Arch
//!
//! Prevents concurrent writers from corrupting `storage.fico` / `storage.sqlite`.
//! Uses `foundation::file::FileLock`, a thin `flock(2)` wrapper, on a `.lock`
//! file beside the storage.

use crate::error::{CoreDataError, Result};
use foundation::file::FileLock;
use std::path::{Path, PathBuf};

pub struct StorageLock {
    lock: FileLock,
}

impl StorageLock {
    /// Acquire exclusive lock for bundle's storage dir.
    pub fn exclusive(bundle_id: &str) -> Result<Self> {
        let dir = crate::paths::ensure_storage_dir(bundle_id)?;
        Self::exclusive_for_storage_dir(&dir)
    }

    /// Acquire exclusive lock for a system-wide storage dir.
    pub fn exclusive_for_system(bundle_id: &str) -> Result<Self> {
        let dir = crate::paths::ensure_system_storage_dir(bundle_id)?;
        Self::exclusive_for_storage_dir(&dir)
    }

    /// Acquire exclusive lock on the `.lock` file inside `dir`.
    pub fn exclusive_for_storage_dir(dir: &Path) -> Result<Self> {
        Ok(Self {
            lock: open_lock_file(dir, LockMode::Exclusive)?,
        })
    }

    pub fn shared(bundle_id: &str) -> Result<Self> {
        let dir = crate::paths::ensure_storage_dir(bundle_id)?;
        Self::shared_for_storage_dir(&dir)
    }

    /// Acquire shared lock for a system-wide storage dir.
    pub fn shared_for_system(bundle_id: &str) -> Result<Self> {
        let dir = crate::paths::ensure_system_storage_dir(bundle_id)?;
        Self::shared_for_storage_dir(&dir)
    }

    /// Acquire shared lock on the `.lock` file inside `dir`.
    pub fn shared_for_storage_dir(dir: &Path) -> Result<Self> {
        Ok(Self {
            lock: open_lock_file(dir, LockMode::Shared)?,
        })
    }

    /// Try to acquire the lock without blocking. Returns `Ok(None)` when
    /// another process already holds it.
    pub fn try_exclusive(bundle_id: &str) -> Result<Option<Self>> {
        let dir = crate::paths::ensure_storage_dir(bundle_id)?;
        let lock_path = dir.join(".lock");
        Ok(FileLock::try_exclusive(&lock_path)
            .map_err(|e| match e {
                foundation::error::FoundationError::Io(e) => CoreDataError::Io(e),
                other => CoreDataError::custom(other.to_string()),
            })?
            .map(|lock| Self { lock }))
    }

    pub fn path(&self) -> &Path {
        self.lock.path()
    }
}

/// Which kind of lock to take on a storage directory.
#[derive(Clone, Copy)]
enum LockMode {
    Exclusive,
    Shared,
}

/// Open `<dir>/.lock` as owner-only and take the requested lock.
fn open_lock_file(dir: &Path, mode: LockMode) -> Result<FileLock> {
    let lock_path = dir.join(".lock");
    std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(CoreDataError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::metadata(&lock_path).map(|m| {
            let mut p = m.permissions();
            p.set_mode(0o600);
            let _ = std::fs::set_permissions(&lock_path, p);
        });
    }
    let result = match mode {
        LockMode::Exclusive => FileLock::exclusive(&lock_path),
        LockMode::Shared => FileLock::shared(&lock_path),
    };
    result.map_err(|e| match e {
        foundation::error::FoundationError::Io(e) => CoreDataError::Io(e),
        other => CoreDataError::custom(other.to_string()),
    })
}

/// The path of a storage directory's lock file.
pub fn lock_path_for(dir: &Path) -> PathBuf {
    dir.join(".lock")
}