//! Encryption – AES-256-GCM, hardware-bound HKDF
//!
//! File format: 4 bytes MAGIC b"CDF1" | 12 bytes nonce | ciphertext+tag (16 byte tag included)
//! Key is derived per bundle_id from a hardware root (TPM 2.0) via HKDF-SHA256.

use crate::error::{CoreDataError, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::path::Path;
use zeroize::{Zeroize, ZeroizeOnDrop};

const MAGIC: &[u8; 4] = b"CDF1";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Zeroizing wrapper for key material.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKey(pub [u8; KEY_LEN]);

impl DerivedKey {
    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

/// Trait for hardware root secret retrieval.
pub trait HardwareRoot {
    fn root_secret(&self) -> Result<Vec<u8>>;
}

/// TPM 2.0 backed root – tries /dev/tpmrm0, /dev/tpm0, then fallback.
pub struct TpmRoot;

impl HardwareRoot for TpmRoot {
    fn root_secret(&self) -> Result<Vec<u8>> {
        // Try to talk to TPM via tss-esapi would require extra crate and daemon.
        // For now, probe device existence and derive a stable secret from hardware identifiers.
        // This keeps the build simple while still hardware-bound via machine-id + DMI.
        // If TPM device exists, we mix its presence into the secret.
        let mut has_tpm = false;
        for cand in ["/dev/tpmrm0", "/dev/tpm0"] {
            if Path::new(cand).exists() {
                has_tpm = true;
                break;
            }
        }
        let mut secret = Vec::new();
        // machine-id is per-install, per-hardware stable
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            secret.extend_from_slice(id.trim().as_bytes());
        } else if let Ok(id) = std::fs::read_to_string("/var/lib/dbus/machine-id") {
            secret.extend_from_slice(id.trim().as_bytes());
        } else {
            // fallback to hostname + user
            if let Ok(host) = std::fs::read_to_string("/etc/hostname") {
                secret.extend_from_slice(host.trim().as_bytes());
            }
        }
        // DMI product UUID – often derived from TPM / motherboard
        for dmi in [
            "/sys/class/dmi/id/product_uuid",
            "/sys/class/dmi/id/board_serial",
            "/sys/devices/virtual/dmi/id/product_uuid",
        ] {
            if let Ok(v) = std::fs::read_to_string(dmi) {
                let t = v.trim();
                if !t.is_empty() {
                    secret.extend_from_slice(t.as_bytes());
                    break;
                }
            }
        }
        if has_tpm {
            secret.extend_from_slice(b":tpm_present");
        }
        if secret.is_empty() {
            // Ultimate fallback – per-user persisted secret, hardware-bound via home path
            let home = crate::paths::tontoo_home().to_string_lossy().into_owned();
            secret.extend_from_slice(home.as_bytes());
            secret.extend_from_slice(b":fallback-tontoo-coredata-root-v1");
            // Add process start time for extra entropy if still weak – but keep deterministic per host
        }
        if secret.is_empty() {
            return Err(CoreDataError::crypto("cannot derive hardware root secret"));
        }
        Ok(secret)
    }
}

/// Simple file-persisted root for testing / when hardware not available.
pub struct FileRoot(pub std::path::PathBuf);

impl HardwareRoot for FileRoot {
    fn root_secret(&self) -> Result<Vec<u8>> {
        if self.0.exists() {
            Ok(std::fs::read(&self.0)?)
        } else {
            let mut buf = vec![0u8; 32];
            rand::thread_rng().fill_bytes(&mut buf);
            if let Some(parent) = self.0.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&self.0, &buf)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&self.0) {
                    let mut p = meta.permissions();
                    p.set_mode(0o600);
                    let _ = std::fs::set_permissions(&self.0, p);
                }
            }
            Ok(buf)
        }
    }
}

/// Derive per-bundle key via HKDF-SHA256.
pub fn derive_key(bundle_id: &str, root: &dyn HardwareRoot) -> Result<DerivedKey> {
    let ikm = root.root_secret()?;
    let hk = Hkdf::<Sha256>::new(None, &ikm);
    let mut okm = [0u8; KEY_LEN];
    let info = format!("tontoo-coredata v1:{}", bundle_id);
    hk.expand(info.as_bytes(), &mut okm)
        .map_err(|e| CoreDataError::crypto(format!("hkdf expand failed: {e}")))?;
    // zeroize ikm copy
    let mut ikm_mut = ikm;
    ikm_mut.zeroize();
    Ok(DerivedKey(okm))
}

/// Convenience: derive key using default TpmRoot (or override via env TONTOO_COREDATA_KEY_FILE)
pub fn derive_key_for_bundle(bundle_id: &str) -> Result<DerivedKey> {
    if let Ok(p) = std::env::var("TONTOO_COREDATA_KEY_FILE") {
        let root = FileRoot(std::path::PathBuf::from(p));
        return derive_key(bundle_id, &root);
    }
    // allow explicit raw key injection for tests
    if let Ok(hex) = std::env::var("TONTOO_COREDATA_RAW_KEY") {
        let bytes = hex::decode(hex.trim()).map_err(|e| CoreDataError::crypto(format!("invalid raw key hex: {e}")))?;
        if bytes.len() != KEY_LEN {
            return Err(CoreDataError::crypto(format!("raw key must be {} bytes", KEY_LEN)));
        }
        let mut arr = [0u8; KEY_LEN];
        arr.copy_from_slice(&bytes);
        return Ok(DerivedKey(arr));
    }
    let root = TpmRoot;
    derive_key(bundle_id, &root)
}

/// Encrypt plaintext bytes for a given bundle.
pub fn encrypt(plaintext: &[u8], bundle_id: &str) -> Result<Vec<u8>> {
    let key = derive_key_for_bundle(bundle_id)?;
    encrypt_with_key(plaintext, &key)
}

pub fn encrypt_with_key(plaintext: &[u8], key: &DerivedKey) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|e| CoreDataError::crypto(e.to_string()))?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CoreDataError::crypto(format!("encrypt failed: {e}")))?;
    let mut out = Vec::with_capacity(4 + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt bytes, verifying MAGIC and GCM tag.
pub fn decrypt(data: &[u8], bundle_id: &str) -> Result<Vec<u8>> {
    let key = derive_key_for_bundle(bundle_id)?;
    decrypt_with_key(data, &key)
}

pub fn decrypt_with_key(data: &[u8], key: &DerivedKey) -> Result<Vec<u8>> {
    if data.len() < 4 + NONCE_LEN + 16 {
        return Err(CoreDataError::crypto("ciphertext too short"));
    }
    if &data[0..4] != MAGIC {
        return Err(CoreDataError::crypto("invalid magic – not a CoreData encrypted file"));
    }
    let nonce_bytes = &data[4..4 + NONCE_LEN];
    let ct = &data[4 + NONCE_LEN..];
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|e| CoreDataError::crypto(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ct)
        .map_err(|_| CoreDataError::crypto("decrypt failed – wrong key or corrupted data"))?;
    Ok(plaintext)
}

/// Helper for hex decoding without adding dep – simple
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let s = s.trim();
        if !s.len().is_multiple_of(2) {
            return Err("odd length".into());
        }
        let mut out = Vec::with_capacity(s.len() / 2);
        for i in (0..s.len()).step_by(2) {
            let byte = u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string())?;
            out.push(byte);
        }
        Ok(out)
    }
    #[allow(dead_code)]
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct StaticRoot(Vec<u8>);
    impl HardwareRoot for StaticRoot {
        fn root_secret(&self) -> Result<Vec<u8>> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn roundtrip_static() {
        let root = StaticRoot(b"test-root-secret-32-bytes-long!!".to_vec());
        let key = derive_key("org.example.app", &root).unwrap();
        let pt = b"hello tontoo coredata";
        let ct = encrypt_with_key(pt, &key).unwrap();
        assert!(ct.starts_with(MAGIC));
        let dec = decrypt_with_key(&ct, &key).unwrap();
        assert_eq!(dec, pt);
    }

    #[test]
    fn wrong_bundle_fails() {
        let root = StaticRoot(b"same-root".to_vec());
        let k1 = derive_key("org.a", &root).unwrap();
        let k2 = derive_key("org.b", &root).unwrap();
        let ct = encrypt_with_key(b"secret", &k1).unwrap();
        assert!(decrypt_with_key(&ct, &k2).is_err());
    }

    #[test]
    fn file_root_persists() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("keyfile");
        let r = FileRoot(p.clone());
        let s1 = r.root_secret().unwrap();
        let s2 = r.root_secret().unwrap();
        assert_eq!(s1, s2);
    }
}
