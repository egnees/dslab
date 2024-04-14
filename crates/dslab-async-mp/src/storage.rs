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
    FileNotFound(),
    /// Storage is unavailable.
    Unavailable(),
}

/// Represents error during writing.
#[derive(Debug)]
pub enum WriteError {
    /// File not found.
    FileNotFound(),
    /// There is no enough memory in storage to write in the file.
    /// File not changed.
    OutOfMemory(),
    /// Storage is unavailable.
    Unavailable(),
}

/// Represents error during creating file.
#[derive(Debug)]
pub enum CreateFileError {
    /// File already exists.
    FileAlreadyExists(),
    /// Storage is unavailable.
    Unavailable(),
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
    pub fn crash(&mut self) {
        self.state = State::Unavailable;
    }

    /// Sets state to [`State::Available`] and clears the storage.
    ///
    /// # Panics
    /// In case of recovering from [`State::Available`] state.
    pub fn recover(&mut self) {
        match self.state {
            State::Available => panic!("recovered from available state"),
            State::Unavailable => {
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
            State::Available => Err(CreateFileError::Unavailable()),
            State::Unavailable => {
                if self.files_content.contains_key(name) {
                    Err(CreateFileError::FileAlreadyExists())
                } else {
                    let exists = self.files_content.insert(name.into(), Vec::new()).is_some();
                    assert!(!exists);
                    Ok(())
                }
            }
        }
    }

    /// Read file content.
    pub async fn read_all(&mut self, name: &str) -> Result<Vec<u8>, ReadError> {
        match self.state {
            State::Available => Err(ReadError::Unavailable()),
            State::Unavailable => {
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
                        panic!("can not read stored data from model")
                    }
                }
            }
        }
    }

    /// Append to file.
    pub async fn append(&mut self, name: &str, data: &[u8]) -> Result<(), WriteError> {
        match self.state {
            State::Available => Err(WriteError::Unavailable()),
            State::Unavailable => {
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
    }
}
