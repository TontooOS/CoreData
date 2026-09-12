//! File lock for storage – exclusive flock on TontooOS/Arch
//!
//! Prevents concurrent writers from corrupting `storage.fico` / `storage.sqlite`.
//! Uses `fs2::FileExt` (flock) on a `.lock` file beside the storage.

use crate::error::{CoreDataError, Result};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub struct StorageLock {
    _file: File,
    path: PathBuf,
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
        let lock_path = dir.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(CoreDataError::Io)?;
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::metadata(&lock_path).map(|m| {
                let mut p = m.permissions();
                p.set_mode(0o600);
                let _ = std::fs::set_permissions(&lock_path, p);
            });
        }
        file.lock_exclusive().map_err(CoreDataError::Io)?;
        Ok(Self { _file: file, path: lock_path })
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
        let lock_path = dir.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(CoreDataError::Io)?;
        file.lock_shared().map_err(CoreDataError::Io)?;
        Ok(Self { _file: file, path: lock_path })
    }

    /// Try to acquire with timeout for tests? For now blocking.
    pub fn try_exclusive(bundle_id: &str) -> Result<Option<Self>> {
        let dir = crate::paths::ensure_storage_dir(bundle_id)?;
        let lock_path = dir.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(CoreDataError::Io)?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { _file: file, path: lock_path })),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(CoreDataError::Io(e)),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StorageLock {
    fn drop(&mut self) {
        let _ = self._file.unlock();
    }
}
