//! FishPerms integration – Storage permission
//!
//! Requirement: No popup for own storage, silent deny for foreign access.
//! We implement a check that uses fishperms daemon if available, otherwise
//! falls back to owner check.
//!
//! Protocol is kept minimal to avoid hard dep on fishperms crates at compile.
//! We speak the same JSON framing as fishperms-protocol but without linking it.

use crate::error::{CoreDataError, Result};
use std::path::PathBuf;

const DEFAULT_SOCKET: &str = "/run/fishperms.sock";

fn socket_path() -> PathBuf {
    std::env::var("FISHPERMS_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SOCKET))
}

/// Check if caller may access storage owned by `owner_bundle_id`.
/// Returns Ok(()) if allowed, Err(PermissionDenied) if not.
/// No prompt is ever shown for Storage – it is silent.
pub fn check_storage_access(owner_bundle_id: &str) -> Result<()> {
    let caller = crate::paths::resolve_caller_id().unwrap_or_else(|_| "unknown".to_string());
    // If caller == owner, always allow (no daemon needed)
    if caller == owner_bundle_id {
        return Ok(());
    }
    // Try fishperms daemon first
    if let Ok(allowed) = check_via_daemon(&caller, owner_bundle_id) {
        if allowed {
            return Ok(());
        } else {
            return Err(CoreDataError::PermissionDenied {
                caller,
                owner: owner_bundle_id.to_string(),
            });
        }
    }
    // Daemon not reachable – enforce isolation: only owner may access
    // If caller is unknown, we allow only if file does not exist yet? No – deny foreign.
    Err(CoreDataError::PermissionDenied {
        caller,
        owner: owner_bundle_id.to_string(),
    })
}

/// Try to ask daemon. Returns Ok(true) allowed, Ok(false) denied, Err if daemon unreachable.
#[cfg(unix)]
fn check_via_daemon(caller: &str, owner: &str) -> std::result::Result<bool, ()> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;
    let path = socket_path();
    if !path.exists() {
        return Err(());
    }
    let mut stream = UnixStream::connect(&path).map_err(|_| ())?;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    let req = serde_json::json!({
        "v": 1,
        "id": 1,
        "method": "check",
        "params": { "permission": "storage", "app": owner, "caller": caller }
    });
    let payload = serde_json::to_vec(&req).map_err(|_| ())?;
    let len = payload.len() as u32;
    stream.write_all(&len.to_le_bytes()).map_err(|_| ())?;
    stream.write_all(&payload).map_err(|_| ())?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|_| ())?;
    let resp_len = u32::from_le_bytes(len_buf) as usize;
    if resp_len > 1024 * 1024 {
        return Err(());
    }
    let mut buf = vec![0u8; resp_len];
    stream.read_exact(&mut buf).map_err(|_| ())?;
    let resp: serde_json::Value = serde_json::from_slice(&buf).map_err(|_| ())?;
    if resp.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        if let Some(result) = resp.get("result") {
            if let Some(s) = result.as_str() {
                return Ok(s == "allowed");
            }
            if let Some(state) = result.get("state").and_then(|v| v.as_str()) {
                return Ok(state == "allowed");
            }
            if let Some(allow) = result.get("allowed").and_then(|v| v.as_bool()) {
                return Ok(allow);
            }
        }
        return Ok(true);
    }
    Ok(false)
}

#[cfg(not(unix))]
fn check_via_daemon(_caller: &str, _owner: &str) -> std::result::Result<bool, ()> {
    Err(())
}

/// No-op register – ensures Storage permission is known to daemon without prompting.
/// Called once at container init for owner.
pub fn ensure_storage_policy(owner_bundle_id: &str) {
    // We do not need to register – isolation is file-based.
    // This hook exists for future daemon that supports Storage registration.
    let _ = owner_bundle_id;
}

/// Helper to decide perms for file access without daemon – used by stores to check before open.
pub fn is_owner(owner_bundle_id: &str) -> bool {
    match crate::paths::resolve_caller_id() {
        Ok(caller) => caller == owner_bundle_id,
        Err(_) => false,
    }
}

/// Strict check that aborts if not owner – used when daemon unreachable.
/// For unit tests, allow bypass via TONTOO_COREDATA_ALLOW_FOREIGN=1
pub fn enforce_owner_or_fail(owner_bundle_id: &str) -> Result<()> {
    enforce_access(owner_bundle_id, false)
}

/// Owner check shared by per-user and system-wide stores.
/// Per-user stores (`system == false`) enforce bundle isolation.
/// System-wide stores (`system == true`) skip the bundle check: the writer
/// is a privileged daemon and isolation is enforced by the 0o700 directory
/// itself, so unprivileged callers fail with an I/O permission error when
/// they cannot open the files.
pub fn enforce_access(owner_bundle_id: &str, system: bool) -> Result<()> {
    if system {
        return Ok(());
    }
    if std::env::var("TONTOO_COREDATA_ALLOW_FOREIGN").map(|v| v == "1").unwrap_or(false) {
        return Ok(());
    }
    let caller_opt = crate::paths::resolve_caller_id().ok();
    match caller_opt {
        Some(caller) if caller != owner_bundle_id => {
            Err(CoreDataError::PermissionDenied { caller, owner: owner_bundle_id.to_string() })
        }
        _ => Ok(()),
    }
}
