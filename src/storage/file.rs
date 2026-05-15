//! File-based storage

use std::fs::{File, OpenOptions, create_dir_all};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use crate::DEFAULT_PAGE_SIZE;

/// File-based storage (single file per index)
pub struct FileStorage {
    file: Option<File>,
    file_path: String,
    page_size: usize,
    next_page_id: u64,
    file_size: u64,
}

impl FileStorage {
    /// Open or create a file storage
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();
        let parent = path.parent();
        if let Some(p) = parent {
            create_dir_all(p)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();

        // Determine page count from file size
        let page_size = DEFAULT_PAGE_SIZE;
        let next_page_id = if file_size > 0 {
            (file_size / page_size as u64) as u64
        } else {
            0
        };

        Ok(Self {
            file: Some(file),
            file_path: path.to_string_lossy().to_string(),
            page_size,
            next_page_id,
            file_size,
        })
    }

    /// Get storage path
    pub fn path(&self) -> &str {
        &self.file_path
    }

    /// Get file size
    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Get page size
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// Get next page id
    pub fn next_page_id(&self) -> u64 {
        self.next_page_id
    }

    /// Set next page id
    pub fn set_next_page_id(&mut self, id: u64) {
        self.next_page_id = id;
    }

    /// Set file size
    pub fn set_file_size(&mut self, size: u64) {
        self.file_size = size;
    }

    /// Get mutable file reference
    pub fn file_mut(&mut self) -> &mut Option<File> {
        &mut self.file
    }
}

impl Default for FileStorage {
    fn default() -> Self {
        Self {
            file: None,
            file_path: String::new(),
            page_size: DEFAULT_PAGE_SIZE,
            next_page_id: 0,
            file_size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_storage() {
        let tmp = NamedTempFile::new().unwrap();
        let mut storage = FileStorage::open(tmp.path()).unwrap();

        let page_id = storage.alloc_page();
        assert_eq!(page_id, 0);
    }

    #[test]
    fn test_file_storage_persist() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        drop(tmp);

        // Write
        {
            let mut storage = FileStorage::open(&path).unwrap();
            let page = crate::page::Page::new_leaf(0);
            storage.write_page(&page);
            storage.flush();
        }

        // Read
        {
            let mut storage = FileStorage::open(&path).unwrap();
            let page = storage.read_page(0).unwrap();
            assert_eq!(page.header.page_id, 0);
        }
    }
}