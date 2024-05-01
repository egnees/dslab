//! Definition of possible errors for storage operations.

/// Represents possible errors for storage operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// Resource can not be created, because it already exists.
    AlreadyExists,
    /// Resource not found.
    NotFound,
    /// Storage is unavailable.
    /// Requested operation can be completed or not.
    Unavailable,
    /// Passed buffer size exceeds limit.
    BufferSizeExceed,
}

/// Represents result of storage operation.
pub type StorageResult<T> = Result<T, StorageError>;
