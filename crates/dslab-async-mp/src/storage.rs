//! Definition of filesystem.

use std::{cell::RefCell, collections::HashMap, fs::File, rc::Rc};

use dslab_core::{Id, Simulation, SimulationContext};
use dslab_storage::disk::{self, DiskBuilder};
use dslab_storage::events::{FileReadCompleted, FileReadFailed};
pub use dslab_storage::{disk::Disk, fs::FileSystem};

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
    fs: Rc<RefCell<FileSystem>>,
    files_content: HashMap<String, Vec<u8>>,
    ctx: SimulationContext,
}

impl Storage {
    /// Creates a new storage.
    pub fn new(fs: Rc<RefCell<FileSystem>>, ctx: SimulationContext) -> Self {
        Self {
            fs,
            files_content: HashMap::new(),
            ctx,
        }
    }

    /// Create file with specified name.
    /// If file with such name already exists,
    /// [`CreateFileError::FileAlreadyExists`] will be returned.
    pub async fn create_file(&mut self, name: &str) -> Result<(), CreateFileError> {
        if let Err(_) = self.fs.borrow_mut().create_file(name) {
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
        let key = self.fs.borrow_mut().read_all(name, self.ctx.id());
        self.ctx.recv_event_by_key::<FileReadCompleted>(key).await;
        return Ok(self.files_content.get(name).unwrap().clone());
    }

    /// Append to file.
    pub async fn append(&mut self, name: &str, data: &[u8]) -> Result<(), WriteError> {
        let key = self
            .fs
            .borrow_mut()
            .write(name, data.len().try_into().unwrap(), self.ctx.id());

        todo!()
    }

    pub fn load_from_dir() {}
}
