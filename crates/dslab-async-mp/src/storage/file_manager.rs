//! Definition of filesystem.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dslab_core::SimulationContext;
pub use dslab_storage::storage::Storage;
use dslab_storage::storage::Storage as StorageModel;

use super::{
    file::File,
    model::ModelWrapper,
    result::{StorageError, StorageResult},
};

/// Represents content of file shared between [`files`][`File`].
pub type SharedFileContent = Rc<RefCell<Vec<u8>>>;

/// Represents file manager, which is responsible for creating and opening files.
pub struct FileManager {
    /// Content of files stored here.
    pub files_content: HashMap<String, SharedFileContent>,
    /// Context of the owner node.
    pub ctx: SimulationContext,
    /// Wrapper of storage model.
    pub storage_wrapper: Rc<RefCell<ModelWrapper>>,
}

impl FileManager {
    /// Creates a new storage.
    pub fn new(model: Rc<RefCell<dyn StorageModel>>, ctx: SimulationContext) -> Self {
        let model_wrapper = ModelWrapper::new(model, ctx.clone());
        Self {
            files_content: HashMap::new(),
            ctx,
            storage_wrapper: Rc::new(RefCell::new(model_wrapper)),
        }
    }

    /// Mark storage as unavailable.
    /// Storage data will not be destroyed until recover will be called.
    pub fn crash_storage(&mut self) {
        self.storage_wrapper.borrow_mut().crash();
        // Data is not destroyed until recover will be called.
    }

    /// Sets state to [`State::Available`] and clears the storage.
    ///
    /// # Panics
    /// In case of recovering from [`State::Available`] state.
    pub fn recover_storage(&mut self) {
        let is_available = self.storage_wrapper.borrow().is_available();
        match is_available {
            true => panic!("trying to recover, but storage is not crashed"),
            false => {
                // Data is destroyed on recovery to allow working with it after crash.

                // Delete files.
                self.files_content.clear();

                // Recover model.
                self.storage_wrapper.borrow_mut().recover();
            }
        }
    }

    /// Create file with specified name.
    /// Created file will have zero space.
    /// If file with such name already exists,
    /// [`StorageError`] will be returned.
    pub fn create_file(&mut self, name: &str) -> StorageResult<File> {
        let is_available = self.storage_wrapper.borrow().is_available();
        match is_available {
            false => Err(StorageError::Unavailable),
            true => {
                if self.files_content.contains_key(name) {
                    Err(StorageError::AlreadyExists)
                } else {
                    let content = Rc::new(RefCell::new(Vec::new()));
                    self.files_content.insert(name.to_string(), content.clone());
                    Ok(File {
                        storage_wrapper: self.storage_wrapper.clone(),
                        content,
                        ctx: self.ctx.clone(),
                    })
                }
            }
        }
    }

    /// Delete file with specified name.
    /// If file with such name not exists,
    /// [`NotFound`][`StorageError::NotFound`] error will be returned.
    pub fn delete_file(&mut self, name: &str) -> StorageResult<()> {
        let is_available = self.storage_wrapper.borrow().is_available();
        match is_available {
            false => Err(StorageError::Unavailable),
            true => {
                let remove_result = self.files_content.remove(name);
                if let Some(_) = remove_result {
                    Ok(())
                } else {
                    Err(StorageError::NotFound)
                }
            }
        }
    }

    /// Check if file with specified name exists.
    pub fn file_exists(&self, name: &str) -> StorageResult<bool> {
        let is_available = self.storage_wrapper.borrow().is_available();
        match is_available {
            false => Err(StorageError::Unavailable),
            true => Ok(self.files_content.contains_key(name)),
        }
    }

    /// Open file with specified name.
    pub fn open_file(&self, name: &str) -> StorageResult<File> {
        let is_available = self.storage_wrapper.borrow().is_available();
        match is_available {
            false => Err(StorageError::Unavailable),
            true => {
                if self.files_content.contains_key(name) {
                    Ok(File {
                        storage_wrapper: self.storage_wrapper.clone(),
                        content: self.files_content.get(name).unwrap().clone(),
                        ctx: self.ctx.clone(),
                    })
                } else {
                    Err(StorageError::NotFound)
                }
            }
        }
    }
}
