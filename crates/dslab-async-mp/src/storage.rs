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
#[derive(Debug)]
pub enum ReadError {
    /// File not found.
    FileNotFound,
    /// Storage is unavailable.
    Unavailable,
}

/// Represents error during writing.
#[derive(Debug)]
pub enum WriteError {
    /// File not found.
    FileNotFound,
    /// There is no enough memory in storage to write in the file.
    /// File not changed.
    OutOfMemory,
    /// Storage is unavailable.
    Unavailable,
}

/// Represents error during creating file.
#[derive(Debug)]
pub enum CreateFileError {
    /// File already exists.
    FileAlreadyExists,
    /// Storage is unavailable.
    Unavailable,
}

/// Represents error during removing file.
#[derive(Debug)]
pub enum DeleteFileError {
    /// File not found.
    FileNotFound,
    /// Storage is unavailable.
    Unavailable,
}

/// Represents state of the storage.
pub enum State {
    /// Storage is able to response on the requests.
    Available,
    /// Storage will response with `Unavailable` error
    /// (see [`CreateFileError::Unavailable`], [`WriteError::Unavailable`], [`ReadError::Unavailable`]).
    Unavailable,
}

/// Represents filesystem mounted on the single disk.
pub struct Storage {
    model: Rc<RefCell<dyn StorageModel>>,
    files_content: HashMap<String, Vec<u8>>,
    ctx: SimulationContext,
    state: State,
}

/// Represents max size of buffer for read request.
pub const MAX_BUFFER_SIZE: usize = 1 << 30; // 1 Gb.

/// Represents typical number of bytes, returned by `[Storage::read]`.
const TYPICAL_READ_SIZE: usize = 2 * (1 << 20); // 2 Mb.

impl Storage {
    /// Creates a new storage.
    pub fn new(disk: Rc<RefCell<dyn StorageModel>>, ctx: SimulationContext) -> Self {
        Self {
            model: disk,
            files_content: HashMap::new(),
            ctx,
            state: State::Available,
        }
    }

    /// Sets state to [`State::Unavailable`].
    /// Storage data will not be destroyed until recover will be called.
    pub fn crash(&mut self) {
        self.state = State::Unavailable;
    }

    /// Sets state to [`State::Available`] and clears the storage.
    ///
    /// # Panics
    /// In case of recovering from [`State::Available`] state.
    pub fn recover(&mut self) {
        match self.state {
            State::Unavailable => panic!("recovery from available state"),
            State::Available => {
                // Data is destroyed on recovery to allow working with it after crash.

                // Delete files.
                self.files_content = HashMap::new();
                self.state = State::Available;

                // Clear model.
                let size = self.model.borrow().used_space();
                self.model.borrow_mut().mark_free(size).unwrap();
            }
        }
    }

    /// Create file with specified name.
    /// If file with such name already exists,
    /// [`CreateFileError::FileAlreadyExists`] will be returned.
    pub async fn create_file(&mut self, name: &str) -> Result<(), CreateFileError> {
        match self.state {
            State::Unavailable => Err(CreateFileError::Unavailable),
            State::Available => {
                if self.files_content.contains_key(name) {
                    Err(CreateFileError::FileAlreadyExists)
                } else {
                    let exists = self.files_content.insert(name.into(), Vec::new()).is_some();
                    assert!(!exists);
                    Ok(())
                }
            }
        }
    }

    /// Delete file with specified name.
    pub async fn delete_file(&mut self, name: &str) -> Result<(), DeleteFileError> {
        match self.state {
            State::Available => {
                if let Some(file) = self.files_content.remove(name) {
                    self.model
                        .borrow_mut()
                        .mark_free(file.len().try_into().unwrap())
                        .unwrap();
                    Ok(())
                } else {
                    Err(DeleteFileError::FileNotFound)
                }
            }
            State::Unavailable => Err(DeleteFileError::Unavailable),
        }
    }

    /// Read file content from the specified offset to the specified destination.
    ///
    /// # Returns
    /// The number of read bytes.
    pub async fn read(&mut self, file: &str, offset: usize, dst: &mut [u8]) -> Result<usize, ReadError> {
        if dst.len() > MAX_BUFFER_SIZE {
            panic!(
                "size of buffer exceeds max size: {} exceeds {}",
                dst.len(),
                MAX_BUFFER_SIZE
            );
        }

        match self.state {
            State::Available => {
                if !self.files_content.contains_key(file) {
                    return Err(ReadError::FileNotFound);
                }
                let content = self.files_content.get(file).unwrap();
                if offset >= content.len() {
                    return Ok(0);
                }
                let copy_len = dst.len().min(content.len() - offset).min(TYPICAL_READ_SIZE);
                dst[..copy_len].copy_from_slice(&content.as_slice()[offset..offset + copy_len]);
                Ok(copy_len)
            }
            State::Unavailable => Err(ReadError::Unavailable),
        }
    }

    /// Read file content.
    pub async fn read_all(&mut self, name: &str) -> Result<Vec<u8>, ReadError> {
        match self.state {
            State::Unavailable => Err(ReadError::Unavailable),
            State::Available => {
                if !self.files_content.contains_key(name) {
                    return Err(ReadError::FileNotFound);
                }

                let content = self.files_content.get(name).unwrap();

                let key = self.model.borrow_mut().read(content.len() as u64, self.ctx.id());
                select! {
                    _ = self.ctx.recv_event_by_key::<DataReadCompleted>(key).fuse() => {
                        Ok(content.clone())
                    }
                    _ = self.ctx.recv_event_by_key::<DataReadFailed>(key).fuse() => {
                        panic!("can not read stored data from model")
                    }
                }
            }
        }
    }

    /// Append to file.
    pub async fn append(&mut self, name: &str, data: &[u8]) -> Result<(), WriteError> {
        match self.state {
            State::Unavailable => Err(WriteError::Unavailable),
            State::Available => {
                if !self.files_content.contains_key(name) {
                    return Err(WriteError::FileNotFound);
                }

                let key = self.model.borrow_mut().write(data.len() as u64, self.ctx.id());
                select! {
                    _ = self.ctx.recv_event_by_key::<DataWriteCompleted>(key).fuse() => {
                        let content = self.files_content.get_mut(name).unwrap();
                        content.extend_from_slice(data);
                        Ok(())
                    }
                    _ = self.ctx.recv_event_by_key::<DataWriteFailed>(key).fuse() => {
                        Err(WriteError::OutOfMemory)
                    }
                }
            }
        }
    }
}
