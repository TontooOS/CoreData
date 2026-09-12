/*
 * Tontoo CoreData – C Header
 * Encrypted, per-app isolated persistence for TontooOS
 * Backends: FishFile .fico + SQLite, AES-256-GCM, hardware-bound key
 * Location: /Users/<user>/Library/Preferences/<bundleId>/storage.{fico,sqlite}
 */

#ifndef COREDATA_H
#define COREDATA_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ======================== */
/* Version */
/* ======================== */

/**
 * Get CoreData library version.
 * @return static version string, e.g. "26.1.0" (do NOT free)
 */
const char* coredata_version(void);

/**
 * Get last error message for current thread.
 * @return allocated string (free with coredata_free_string) or NULL
 */
char* coredata_last_error(void);

/**
 * Free a string returned by coredata_* functions.
 * @param s string to free
 */
void coredata_free_string(char *s);

/* ======================== */
/* Container */
/* ======================== */

/** Opaque handle to a PersistentContainer. */
typedef struct ContainerHandle ContainerHandle;

/**
 * Open a container for a bundle.
 *
 * Bundle id is taken as provided; if you pass the same bundle as the running .app,
 * access is silently allowed. Foreign bundles are denied without popup (FishPerms Storage policy).
 * Storage is encrypted with a hardware-bound key (TPM 2.0 / machine-id).
 *
 * @param bundle_id bundle identifier, e.g. "org.tontooos.dock"
 * @param store_type "fico" or "sqlite" (case-insensitive)
 * @return handle or NULL on error (check coredata_last_error)
 */
ContainerHandle* coredata_open(const char *bundle_id, const char *store_type);

/**
 * Open a system-wide container shared by all users.
 * Storage is encrypted at /System/Preferences/<bundle_id>/storage.{fico,sqlite}.
 * Only privileged writers can create it; unprivileged callers fail with an
 * I/O permission error.
 *
 * @param bundle_id bundle identifier, e.g. "com.tontoo.wifi"
 * @param store_type "fico" or "sqlite" (case-insensitive)
 * @return handle or NULL on error (check coredata_last_error)
 */
ContainerHandle* coredata_open_system(const char *bundle_id, const char *store_type);

/**
 * Close and free a container.
 * @param handle container handle
 */
void coredata_close(ContainerHandle *handle);

/**
 * Persist all pending changes (encrypt + atomic write).
 * @param handle container handle
 * @return 0 on success, -1 on error
 */
int coredata_save(ContainerHandle *handle);

/* ======================== */
/* Object API (C)           */
/* ======================== */

/**
 * Set a value for an object. Creates object if not exists.
 * Value is JSON-encoded: pass "42", "\"hello\"", "true", "null", "[1,2]", "{\"a\":1}"
 * Bare strings without JSON quotes are treated as plain string.
 *
 * @param handle container handle
 * @param entity entity name, e.g. "Note"
 * @param object_id object id (UUID string)
 * @param key attribute name
 * @param json_value JSON value or plain string
 * @return 0 on success, -1 on error
 */
int coredata_set(ContainerHandle *handle, const char *entity, const char *object_id, const char *key, const char *json_value);

/**
 * Get a value by key.
 * @param handle container handle
 * @param entity entity name
 * @param object_id object id
 * @param key attribute name
 * @return allocated JSON string (free with coredata_free_string) or NULL
 */
char* coredata_get(ContainerHandle *handle, const char *entity, const char *object_id, const char *key);

/**
 * Delete an object (soft delete, tombstone kept for future sync).
 * @param handle container handle
 * @param entity entity name (ignored, kept for API symmetry)
 * @param object_id object id
 * @return 0 on success, -1 on error
 */
int coredata_delete(ContainerHandle *handle, const char *entity, const char *object_id);

/**
 * Count objects for an entity.
 * @param handle container handle
 * @param entity entity name
 * @return count or -1 on error
 */
int coredata_fetch_count(ContainerHandle *handle, const char *entity);

#ifdef __cplusplus
}
#endif

#endif /* COREDATA_H */
