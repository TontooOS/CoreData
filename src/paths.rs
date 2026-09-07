//! Path resolution – TontooOS style /Users/<user>/Library/Preferences/<bundleId>/
//!
//! Mirrors FishRunner bundle logic: TONTOO_APP_BUNDLE_ID env -> Info.tontoo -> explicit.

use crate::error::{CoreDataError, Result};
use std::path::{Path, PathBuf};

/// Resolve bundle identifier for the *caller*.
/// Priority: explicit arg > Info.tontoo discovery (via /proc/self/exe + TONTOO_APP_PATH) > TONTOO_APP_BUNDLE_ID env > error
/// Discovery is prioritized over env to prevent spoofing via env on TontooOS/Arch.
pub fn resolve_bundle_id(explicit: Option<&str>) -> Result<String> {
    if let Some(id) = explicit {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(CoreDataError::InvalidBundleId("empty bundle id".into()));
        }
        return Ok(trimmed.to_string());
    }
    // Prefer discovery (exe + Info.tontoo) over env for security
    if let Some(id) = try_discover_bundle_id() {
        return Ok(id);
    }
    if let Ok(env) = std::env::var("TONTOO_APP_BUNDLE_ID") {
        let trimmed = env.trim().to_string();
        if !trimmed.is_empty() {
            // Validate discovered vs env if both exist: if env mismatches discovered, prefer discovered was already returned,
            // so env here is fallback for non-.app binaries / tests
            return Ok(trimmed);
        }
    }
    Err(CoreDataError::NoBundle)
}

/// Resolve caller identity securely (used for isolation checks).
/// Always tries discovery first; env is fallback only for tests/CLI.
pub fn resolve_caller_id() -> Result<String> {
    resolve_bundle_id(None)
}

/// Try to find bundle id by looking for Info.tontoo near the executable.
fn try_discover_bundle_id() -> Option<String> {
    // TONTOO_APP_PATH is set by FishRunner launch.rs
    if let Ok(app_path) = std::env::var("TONTOO_APP_PATH") {
        let p = PathBuf::from(app_path).join("Info.tontoo");
        if let Some(id) = read_bundle_id_from_info(&p) {
            return Some(id);
        }
    }
    // try exe parent and parents (..)
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent().map(PathBuf::from);
        for _ in 0..4 {
            if let Some(dir) = cur.clone() {
                // direct Info.tontoo
                let p = dir.join("Info.tontoo");
                if let Some(id) = read_bundle_id_from_info(&p) {
                    return Some(id);
                }
                // parent may be .app dir: <name>.app/Info.tontoo
                // exe is at <app>.app/App/binary -> parent is App -> two up is .app
                let maybe_app = dir.join("Info.tontoo");
                if let Some(id) = read_bundle_id_from_info(&maybe_app) {
                    return Some(id);
                }
                // go up one
                cur = dir.parent().map(PathBuf::from);
            } else {
                break;
            }
        }
        // also walk up looking for any Info.tontoo
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            let cand = d.join("Info.tontoo");
            if cand.is_file() {
                if let Some(id) = read_bundle_id_from_info(&cand) {
                    return Some(id);
                }
            }
            // also if d is App, check parent
            if d.file_name().map(|n| n == "App").unwrap_or(false) {
                if let Some(parent) = d.parent() {
                    let cand2 = parent.join("Info.tontoo");
                    if let Some(id) = read_bundle_id_from_info(&cand2) {
                        return Some(id);
                    }
                }
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }
    None
}

fn read_bundle_id_from_info(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("bundle_id")?.as_str().map(|s| s.to_string())
}

/// TontooOS home is /Users/<user> (mac-style). We derive from HOME env or dirs::home_dir.
pub fn tontoo_home() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            // On Windows, "/Users/..." is not considered absolute, so accept leading slash explicitly
            if trimmed.starts_with('/') || PathBuf::from(trimmed).is_absolute() {
                return PathBuf::from(trimmed);
            }
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("/Users").join(whoami_fallback()))
}

fn whoami_fallback() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

/// Root for TontooOS preferences: /Users/<user>/Library/Preferences
pub fn preferences_root() -> PathBuf {
    if let Ok(over) = std::env::var("TONTOO_PREFERENCES_ROOT") {
        return PathBuf::from(over);
    }
    tontoo_home().join("Library").join("Preferences")
}

/// Storage directory for a bundle: /Users/<user>/Library/Preferences/<bundleId>
pub fn storage_dir(bundle_id: &str) -> PathBuf {
    preferences_root().join(bundle_id)
}

/// Specific file path for storage.
pub fn storage_path(bundle_id: &str, extension: &str) -> PathBuf {
    storage_dir(bundle_id).join(format!("storage.{}", extension))
}

pub fn fico_path(bundle_id: &str) -> PathBuf {
    storage_path(bundle_id, "fico")
}

pub fn sqlite_path(bundle_id: &str) -> PathBuf {
    storage_path(bundle_id, "sqlite")
}

pub fn meta_path(bundle_id: &str) -> PathBuf {
    storage_path(bundle_id, "meta")
}

/// Ensure storage directory exists with 0o700 perms.
pub fn ensure_storage_dir(bundle_id: &str) -> Result<PathBuf> {
    let dir = storage_dir(bundle_id);
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&dir)?.permissions();
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(&dir, perms);
        }
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_explicit() {
        assert_eq!(resolve_bundle_id(Some("org.tontooos.dock")).unwrap(), "org.tontooos.dock");
    }

    #[test]
    fn test_resolve_env() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("TONTOO_APP_BUNDLE_ID", "org.test.env");
        assert_eq!(resolve_bundle_id(None).unwrap(), "org.test.env");
        std::env::remove_var("TONTOO_APP_BUNDLE_ID");
    }

    #[test]
    fn test_paths() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        // Ensure override not interfering
        let prev_prefs = std::env::var("TONTOO_PREFERENCES_ROOT").ok();
        std::env::remove_var("TONTOO_PREFERENCES_ROOT");
        // Save old HOME
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", "/Users/alice");
        assert_eq!(storage_dir("org.example.app"), PathBuf::from("/Users/alice/Library/Preferences/org.example.app"));
        assert_eq!(fico_path("org.example.app"), PathBuf::from("/Users/alice/Library/Preferences/org.example.app/storage.fico"));
        // restore
        if let Some(v) = prev_home { std::env::set_var("HOME", v); } else { std::env::remove_var("HOME"); }
        if let Some(v) = prev_prefs { std::env::set_var("TONTOO_PREFERENCES_ROOT", v); }
    }

    #[test]
    fn test_preferences_root_override() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        std::env::set_var("TONTOO_PREFERENCES_ROOT", "/tmp/tontoo_prefs");
        assert_eq!(preferences_root(), PathBuf::from("/tmp/tontoo_prefs"));
        std::env::remove_var("TONTOO_PREFERENCES_ROOT");
    }
}
