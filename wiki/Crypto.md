# Crypto

Hardware-bound AES-256-GCM encryption for all stores.

## Format

```
[MAGIC 4][NONCE 12][CIPHERTEXT+TAG]
MAGIC = b"CDF1"
NONCE = 12 bytes random
TAG   = 16 bytes GCM tag (appended by aes-gcm crate)
```

On decrypt, verifies `MAGIC` and GCM tag. Returns `Crypto` error on wrong key or corruption.

## Key Derivation

Per-bundle key via `HKDF-SHA256`:

```
IKM = root_secret() // hardware root
OKM = HKDF(IKM, info="tontoo-coredata v1:{bundle_id}", len=32)
```

### `HardwareRoot` Trait

```rust
pub trait HardwareRoot { fn root_secret(&self) -> Result<Vec<u8>>; }
```

### `TpmRoot`

Probes `/dev/tpmrm0`, `/dev/tpm0`, then `machine-id` (`/etc/machine-id`), then DMI `product_uuid`. Mixes `":tpm_present"` if TPM device exists. Fallback to static string on bare systems. No external `tss-esapi` dependency to keep build simple.

### `FileRoot`

For tests via `TONTOO_COREDATA_KEY_FILE` env: reads/persists 32 random bytes at given path with `0600`. Also used if `TONTOO_COREDATA_RAW_KEY` hex set.

### `derive_key`

```rust
pub fn derive_key(bundle_id: &str, root: &dyn HardwareRoot) -> Result<DerivedKey>
pub fn derive_key_for_bundle(bundle_id: &str) -> Result<DerivedKey>
```

Checks `TONTOO_COREDATA_KEY_FILE` > `TONTOO_COREDATA_RAW_KEY` > `TpmRoot`.

## Encrypt / Decrypt

```rust
pub fn encrypt(plaintext: &[u8], bundle_id: &str) -> Result<Vec<u8>>
pub fn decrypt(data: &[u8], bundle_id: &str) -> Result<Vec<u8>>
pub fn encrypt_with_key(plaintext: &[u8], key: &DerivedKey) -> Result<Vec<u8>>
pub fn decrypt_with_key(data: &[u8], key: &DerivedKey) -> Result<Vec<u8>>
```

Zeroizes `IKM` after HKDF.

## Errors

- `Crypto("ciphertext too short")` if `< 32` bytes
- `Crypto("invalid magic")` if not `CDF1`
- `Crypto("decrypt failed")` on tag failure

## Usage / Example

```rust
let pt = b"hello";
let ct = coredata::crypto::encrypt(pt, "org.example.app").unwrap();
assert!(ct.starts_with(b"CDF1"));
let dec = coredata::crypto::decrypt(&ct, "org.example.app").unwrap();
assert_eq!(pt.to_vec(), dec);
```

## Cross References

- [Paths.md](Paths.md) – bundle id source
- [FicoStore.md](FicoStore.md) – encrypts FishDocument string
- [SqliteStore.md](SqliteStore.md) – encrypts JSON blob per row
