use std::collections::HashMap;

#[derive(Default)]
pub struct Filesystem {
    files: HashMap<String, String>,
}

impl Filesystem {
    pub fn contains_file(&self, filename: &str) -> bool {
        self.files.contains_key(filename)
    }

    pub fn can_be_read(&self, filename: &str, offset: usize, len: usize) -> bool {
        if self.files.contains_key(filename) {
            let file = self.files.get(filename).unwrap();
            offset + len <= file.len()
        } else {
            false
        }
    }

    pub fn create_file(&mut self, filename: &str) -> Result<(), String> {
        if self.files.contains_key(filename) {
            Err("File already exists".to_string())
        } else {
            self.files.insert(filename.to_owned(), "".to_string());
            Ok(())
        }
    }

    pub fn read_file(&self, filename: &str, offset: usize, len: usize) -> Result<String, String> {
        if self.files.contains_key(filename) {
            let file = self.files.get(filename).unwrap();
            if offset + len <= file.len() {
                Ok(file[offset..(offset + len)].to_string())
            } else {
                Err("Invalid offset".to_string())
            }
        } else {
            Err("File does not exists.".to_owned())
        }
    }

    pub fn append_to_file(&mut self, filename: &str, info: &str) -> Result<usize, String> {
        if self.files.contains_key(filename) {
            let file = self.files.get_mut(filename).unwrap();
            let file_size = file.len();
            file.push_str(info);

            Ok(file_size)
        } else {
            Err("File does not exists.".to_string())
        }
    }
}
