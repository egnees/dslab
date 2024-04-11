//! Definition of filesystem.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dslab_core::SimulationContext;
pub use dslab_storage::{disk::Disk, fs::FileSystem};
use dslab_storage::{
    events::{DataReadCompleted, DataReadFailed, DataWriteCompleted, DataWriteFailed},
    storage::Storage as StorageModel,
};

use futures::{select, FutureExt};

/// Represents error during reading.
pub enum ReadError {
    /// File not found.
    FileNotFound(),
}

/// Represents error during writing.
pub enum WriteError {
    /// File not found.
    FileNotFound(),
    /// There is no enough memory in storage to write in the file.
    /// File not changed.
    OutOfMemory(),
}

/// Represents error during creating file.
pub enum CreateFileError {
    /// File already exists.
    FileAlreadyExists(),
}

/// Represents filesystem mounted on the single disk.
pub struct Storage {
    model: Rc<RefCell<dyn StorageModel>>,
    files_content: HashMap<String, Vec<u8>>,
    ctx: SimulationContext,
}

impl Storage {
    /// Creates a new storage.
    pub fn new(disk: Rc<RefCell<dyn StorageModel>>, ctx: SimulationContext) -> Self {
        Self {
            model: disk,
            files_content: HashMap::new(),
            ctx,
        }
    }

    /// Create file with specified name.
    /// If file with such name already exists,
    /// [`CreateFileError::FileAlreadyExists`] will be returned.
    pub async fn create_file(&mut self, name: &str) -> Result<(), CreateFileError> {
        if self.files_content.contains_key(name) {
            Err(CreateFileError::FileAlreadyExists())
        } else {
            let exists = self.files_content.insert(name.into(), Vec::new()).is_some();
            assert!(!exists);
            Ok(())
        }
    }

    /// Read file content.
    pub async fn read_all(&mut self, name: &str) -> Result<Vec<u8>, ReadError> {
        if !self.files_content.contains_key(name) {
            return Err(ReadError::FileNotFound());
        }

        let content = self.files_content.get(name).unwrap();

        let key = self.model.borrow_mut().read(content.len() as u64, self.ctx.id());
        select! {
            _ = self.ctx.recv_event_by_key::<DataReadCompleted>(key).fuse() => {
                Ok(content.clone())
            }
            _ = self.ctx.recv_event_by_key::<DataReadFailed>(key).fuse() => {
                panic!("can not read stored data from disk")
            }
        }
    }

    /// Append to file.
    pub async fn append(&mut self, name: &str, data: &[u8]) -> Result<(), WriteError> {
        if !self.files_content.contains_key(name) {
            return Err(WriteError::FileNotFound());
        }

        let key = self.model.borrow_mut().write(data.len() as u64, self.ctx.id());
        select! {
            _ = self.ctx.recv_event_by_key::<DataWriteCompleted>(key).fuse() => {
                let content = self.files_content.get_mut(name).unwrap();
                content.extend_from_slice(data);
                Ok(())
            }
            _ = self.ctx.recv_event_by_key::<DataWriteFailed>(key).fuse() => {
                Err(WriteError::OutOfMemory())
            }
        }
    }
}
