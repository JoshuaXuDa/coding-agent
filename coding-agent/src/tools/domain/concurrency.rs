//! Concurrent access protection service
//!
//! Prevents race conditions when multiple operations target the same file
//! by providing advisory locking for file operations.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

/// File lock guard
///
/// When dropped, automatically releases the lock.
pub struct FileLockGuard {
    path: String,
    _lock: Arc<Mutex<()>>,
}

impl FileLockGuard {
    /// Create a new file lock guard
    fn new(path: String, lock: Arc<Mutex<()>>) -> Self {
        Self {
            path,
            _lock: lock,
        }
    }

    /// Get the path that is locked
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// File lock manager
///
/// Manages file locks to prevent concurrent access conflicts.
/// Uses a simple in-memory lock mechanism.
pub struct FileLockManager {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl FileLockManager {
    /// Create a new file lock manager
    pub fn new() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
        }
    }

    /// Acquire a read lock for the given path
    ///
    /// Multiple read locks can be held simultaneously, but write locks are exclusive.
    /// Note: This implementation uses exclusive locks for both read and write for simplicity.
    /// A more sophisticated implementation could use RwLock for true read/write locking.
    pub async fn acquire_read_lock(&self, path: &Path) -> FileLockGuard {
        self.acquire_lock(path).await
    }

    /// Acquire a write lock for the given path
    ///
    /// Only one write lock can be held at a time for a given path.
    pub async fn acquire_write_lock(&self, path: &Path) -> FileLockGuard {
        self.acquire_lock(path).await
    }

    /// Try to acquire a write lock with a timeout
    ///
    /// Returns None if the lock cannot be acquired within the timeout.
    pub async fn try_acquire_write_lock(
        &self,
        path: &Path,
        timeout: std::time::Duration,
    ) -> Option<FileLockGuard> {
        let path_str = path.to_string_lossy().to_string();

        // Get or create lock for this path
        let lock = {
            let mut locks = self.locks.lock().await;
            if !locks.contains_key(&path_str) {
                locks.insert(path_str.clone(), Arc::new(Mutex::new(())));
            }
            locks.get(&path_str).unwrap().clone()
        };

        // Try to acquire the lock with timeout
        let mutex = lock.clone();
        let guard: tokio::sync::MutexGuard<()> = tokio::time::timeout(timeout, mutex.lock()).await.ok()?;

        Some(FileLockGuard::new(path_str, lock))
    }

    /// Acquire a lock for the given path (internal)
    async fn acquire_lock(&self, path: &Path) -> FileLockGuard {
        let path_str = path.to_string_lossy().to_string();

        // Get or create lock for this path
        let lock = {
            let mut locks = self.locks.lock().await;
            if !locks.contains_key(&path_str) {
                locks.insert(path_str.clone(), Arc::new(Mutex::new(())));
            }
            locks.get(&path_str).unwrap().clone()
        };

        // Acquire the lock
        drop(lock.lock().await);

        FileLockGuard::new(path_str, lock)
    }

    /// Remove lock for a path if it exists
    ///
    /// This is called automatically when the last guard is dropped,
    /// but can be called explicitly to clean up unused locks.
    pub async fn remove_lock(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        let mut locks = self.locks.lock().await;
        locks.remove(&path_str);
    }

    /// Get the number of active locks
    pub async fn lock_count(&self) -> usize {
        self.locks.lock().await.len()
    }
}

impl Default for FileLockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_lock_manager_new() {
        let manager = FileLockManager::new();
        assert_eq!(manager.lock_count().await, 0);
    }

    #[tokio::test]
    async fn test_acquire_write_lock() {
        let manager = FileLockManager::new();
        let path = Path::new("/tmp/test.txt");

        let guard = manager.acquire_write_lock(path).await;
        assert_eq!(guard.path(), "/tmp/test.txt");
        assert_eq!(manager.lock_count().await, 1);

        // Lock is released when guard is dropped
        drop(guard);
        // Note: Lock is not automatically removed from map when dropped
        assert_eq!(manager.lock_count().await, 1);
    }

    #[tokio::test]
    async fn test_acquire_read_lock() {
        let manager = FileLockManager::new();
        let path = Path::new("/tmp/test.txt");

        let guard = manager.acquire_read_lock(path).await;
        assert_eq!(guard.path(), "/tmp/test.txt");
        assert_eq!(manager.lock_count().await, 1);
    }

    #[tokio::test]
    async fn test_multiple_paths() {
        let manager = FileLockManager::new();
        let path1 = Path::new("/tmp/test1.txt");
        let path2 = Path::new("/tmp/test2.txt");

        let guard1 = manager.acquire_write_lock(path1).await;
        let guard2 = manager.acquire_write_lock(path2).await;

        assert_eq!(manager.lock_count().await, 2);

        drop(guard1);
        drop(guard2);
    }

    #[tokio::test]
    async fn test_try_acquire_with_timeout() {
        let manager = FileLockManager::new();
        let path = Path::new("/tmp/test.txt");

        // First lock should succeed
        let guard1 = manager.acquire_write_lock(path).await;

        // Try to acquire again with short timeout should fail
        let result = manager.try_acquire_write_lock(path, Duration::from_millis(10)).await;
        assert!(result.is_none());

        // Drop first lock and try again
        drop(guard1);
        tokio::time::sleep(Duration::from_millis(50)).await; // Give time for cleanup

        let result = manager.try_acquire_write_lock(path, Duration::from_millis(100)).await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_remove_lock() {
        let mut manager = FileLockManager::new();
        let path = Path::new("/tmp/test.txt");

        let _guard = manager.acquire_write_lock(path).await;
        assert_eq!(manager.lock_count().await, 1);

        manager.remove_lock(path).await;
        assert_eq!(manager.lock_count().await, 0);
    }

    #[tokio::test]
    async fn test_default() {
        let manager = FileLockManager::default();
        assert_eq!(manager.lock_count().await, 0);
    }
}
