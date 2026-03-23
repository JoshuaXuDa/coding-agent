//! Async/Sync bridge service
//!
//! Properly handles async/sync boundaries to avoid blocking runtime issues.
//! This service provides utilities for executing synchronous operations within
//! async contexts without blocking the async executor.

use anyhow::Result;

/// Execute a synchronous operation in an async context
///
/// This function runs the given synchronous operation on a blocking thread pool,
/// preventing it from blocking the async executor. This is essential for CPU-bound
/// or blocking I/O operations that need to be called from async code.
///
/// # Example
/// ```ignore
/// let result = execute_blocking(|| {
///     // Some blocking operation
///     std::fs::read_to_string("/path/to/file")?
/// }).await?;
/// ```
pub async fn execute_blocking<F, R>(operation: F) -> Result<R>
where
    F: FnOnce() -> Result<R> + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
}

/// Execute a synchronous operation with a timeout
///
/// Similar to `execute_blocking` but with a timeout. Returns an error if the
/// operation takes longer than the specified duration.
///
/// # Example
/// ```ignore
/// let result = execute_blocking_with_timeout(
///     Duration::from_secs(5),
///     || {
///         // Some potentially slow operation
///         std::fs::read_to_string("/path/to/file")?
///     }
/// ).await?;
/// ```
pub async fn execute_blocking_with_timeout<F, R>(
    timeout: std::time::Duration,
    operation: F,
) -> Result<R>
where
    F: FnOnce() -> Result<R> + Send + 'static,
    R: Send + 'static,
{
    tokio::time::timeout(timeout, tokio::task::spawn_blocking(operation))
        .await
        .map_err(|_| anyhow::anyhow!("Operation timed out after {:?}", timeout))?
        .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?
}

/// Execute multiple blocking operations concurrently
///
/// Runs multiple blocking operations in parallel on the blocking thread pool.
/// Returns results in the same order as the input operations.
///
/// # Example
/// ```ignore
/// let results = execute_blocking_parallel(vec![
///     || std::fs::read_to_string("/path/to/file1"),
///     || std::fs::read_to_string("/path/to/file2"),
///     || std::fs::read_to_string("/path/to/file3"),
/// ]).await?;
/// ```
pub async fn execute_blocking_parallel<F, R>(operations: Vec<F>) -> Result<Vec<R>>
where
    F: FnOnce() -> Result<R> + Send + 'static,
    R: Send + 'static,
{
    let futures: Vec<_> = operations
        .into_iter()
        .map(|op| tokio::task::spawn_blocking(op))
        .collect();

    let mut results = Vec::with_capacity(futures.len());

    for future in futures {
        let result = future
            .await
            .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?;
        results.push(result?);
    }

    Ok(results)
}

/// A wrapper for values that can be either sync or async
///
/// This enum helps with APIs that may be called from either sync or async contexts.
pub enum AsyncMaybe<R> {
    Sync(Result<R>),
    Async(tokio::task::JoinHandle<Result<R>>),
}

impl<R> AsyncMaybe<R>
where
    R: Send + 'static,
{
    /// Create a sync variant
    pub fn sync(result: Result<R>) -> Self {
        Self::Sync(result)
    }

    /// Create an async variant
    pub fn async_task<F>(operation: F) -> Self
    where
        F: FnOnce() -> Result<R> + Send + 'static,
    {
        Self::Async(tokio::task::spawn_blocking(operation))
    }

    /// Get the result, waiting if necessary
    pub async fn get(self) -> Result<R> {
        match self {
            AsyncMaybe::Sync(result) => result,
            AsyncMaybe::Async(handle) => handle
                .await
                .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::thread;

    #[tokio::test]
    async fn test_execute_blocking_success() {
        let result = execute_blocking(|| {
            // Simulate some blocking work
            thread::sleep(Duration::from_millis(10));
            Ok(42)
        })
        .await
        .unwrap();

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_execute_blocking_error() {
        let result = execute_blocking::<(), i32>(|| {
            Err(anyhow::anyhow!("Test error"))
        })
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Test error");
    }

    #[tokio::test]
    async fn test_execute_blocking_with_timeout_success() {
        let result = execute_blocking_with_timeout(
            Duration::from_secs(1),
            || {
                thread::sleep(Duration::from_millis(10));
                Ok(42)
            },
        )
        .await
        .unwrap();

        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_execute_blocking_with_timeout_exceeded() {
        let result = execute_blocking_with_timeout::<(), i32>(
            Duration::from_millis(10),
            || {
                thread::sleep(Duration::from_millis(100));
                Ok(42)
            },
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_execute_blocking_parallel() {
        let results = execute_blocking_parallel(vec![
            || {
                thread::sleep(Duration::from_millis(10));
                Ok(1)
            },
            || {
                thread::sleep(Duration::from_millis(10));
                Ok(2)
            },
            || {
                thread::sleep(Duration::from_millis(10));
                Ok(3)
            },
        ])
        .await
        .unwrap();

        assert_eq!(results, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_execute_blocking_parallel_with_errors() {
        let results = execute_blocking_parallel::<(), i32>(vec![
            || Ok(1),
            || Err(anyhow::anyhow!("Error 2")),
            || Ok(3),
        ])
        .await;

        assert!(results.is_err());
    }

    #[tokio::test]
    async fn test_async_maybe_sync() {
        let async_maybe = AsyncMaybe::sync(Ok(42));
        let result = async_maybe.get().await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_async_maybe_async() {
        let async_maybe = AsyncMaybe::async_task(|| {
            thread::sleep(Duration::from_millis(10));
            Ok(42)
        });
        let result = async_maybe.get().await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_async_maybe_async_error() {
        let async_maybe = AsyncMaybe::async_task::<(), i32>(|| {
            Err(anyhow::anyhow!("Test error"))
        });
        let result = async_maybe.get().await;
        assert!(result.is_err());
    }
}
