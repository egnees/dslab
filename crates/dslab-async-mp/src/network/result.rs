//! Definition of possible errors for storage operations.

/// Represents possible errors for storage operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    /// Message was not acknowledged in the given time.
    Timeout,
    /// Message was not sent.
    NotSent,
}

/// Represents result of storage operation.
pub type SendResult<T> = Result<T, SendError>;
