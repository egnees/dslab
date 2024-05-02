//! Definition of file of disk.

use std::{cell::RefCell, rc::Rc};

use dslab_core::SimulationContext;
use dslab_storage::events::{DataReadCompleted, DataReadFailed, DataWriteCompleted, DataWriteFailed};
use futures::{select, FutureExt};

use crate::storage::event::StorageCrashedRequestInterrupt;

use super::{
    file_manager::SharedFileContent,
    model::ModelWrapper,
    result::{StorageError, StorageResult},
};

/// Represents file on disk.
pub struct File {
    /// Represents wrapper for storage.
    pub storage_wrapper: Rc<RefCell<ModelWrapper>>,
    /// Represents shared content of the file.
    pub content: SharedFileContent,
    /// Represents context of the owner node.
    pub ctx: SimulationContext,
}

/// Represents maximum size of buffer, passed to io-operation.
pub const MAX_BUFFER_SIZE: u64 = 0x7ffff000;

impl File {
    /// Atomically append bytes from [`data`] to the end of the open file.
    /// # Returns
    /// - Number of appended bytes in case of success.
    ///     * The number of bytes can be less than the [`data`] size, because of lack of storage space.
    /// - [`StorageError`] in case of fail.
    pub async fn append<'a>(&'a mut self, data: &'a [u8]) -> StorageResult<u64> {
        if !self.storage_wrapper.borrow().is_available() {
            return Err(StorageError::Unavailable);
        }

        let buf_size = data.len() as u64;
        if buf_size >= MAX_BUFFER_SIZE {
            return Err(StorageError::BufferSizeExceed);
        }

        let available_size = self.storage_wrapper.borrow().free_space();

        let bytes_to_write = buf_size.min(available_size);
        if bytes_to_write == 0 {
            return Ok(0);
        }

        let request_id = self.storage_wrapper.borrow_mut().write(bytes_to_write);

        select! {
            _ = self.ctx.recv_event_by_key::<DataWriteCompleted>(request_id).fuse() => {
                self.content
                .borrow_mut()
                .extend_from_slice(&data[..bytes_to_write as usize]);

                Ok(bytes_to_write)
            },
            (_, e) = self.ctx.recv_event_by_key::<DataWriteFailed>(request_id).fuse() => {
                panic!("unexpected data write fail: {}", e.error)
            },
            _ = self.ctx.recv_event_by_key::<StorageCrashedRequestInterrupt>(request_id).fuse() => {
                Err(StorageError::Unavailable)
            }
        }
    }

    /// Atomically read bytes from the open file.
    /// # Returns
    /// - Number of bytes read in case of success.
    ///     * This number can be less than buffer size in case of file size not enough.
    /// - [`StorageError`] in case of fail.
    pub async fn read<'a>(&'a mut self, offset: u64, buf: &'a mut [u8]) -> StorageResult<u64> {
        if !self.storage_wrapper.borrow().is_available() {
            return Err(StorageError::Unavailable);
        }

        let buf_size = u64::try_from(buf.len()).unwrap();
        if buf_size >= MAX_BUFFER_SIZE {
            return Err(StorageError::BufferSizeExceed);
        }

        let bytes_to_read = (self.content.borrow().len() as u64)
            .checked_sub(offset)
            .unwrap_or(0)
            .min(buf.len() as u64);

        let request_id = self.storage_wrapper.borrow_mut().read(bytes_to_read);

        select! {
            _ = self.ctx.recv_event_by_key::<DataReadCompleted>(request_id).fuse() => {
                let start = offset as usize;
                let end = start + bytes_to_read as usize;
                buf[..bytes_to_read as usize].copy_from_slice(&self.content.borrow().as_slice()[start..end]);
                Ok(bytes_to_read)
            },
            (_, e) = self.ctx.recv_event_by_key::<DataReadFailed>(request_id).fuse() => {
                panic!("unexpected data read fail: {}", e.error)
            },
            _ = self.ctx.recv_event_by_key::<StorageCrashedRequestInterrupt>(request_id).fuse() => {
                Err(StorageError::Unavailable)
            }
        }
    }
}
