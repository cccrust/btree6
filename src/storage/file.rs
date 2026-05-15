//! File-based storage

use crate::DEFAULT_PAGE_SIZE;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions, create_dir_all};
use std::path::Path;

const FILE_MAGIC: u32 = 0x42545637; // "BTV7"

/// File header metadata (stored in page 0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BTreeHeader {
    pub magic: u32,
    pub version: u32,
    pub root_page: u64,
    pub page_count: u64,
    pub size: u64, // number of key-value pairs
}

impl BTreeHeader {
    pub fn new(root_page: u64, page_count: u64, size: u64) -> Self {
        Self {
            magic: FILE_MAGIC,
            version: 1,
            root_page,
            page_count,
            size,
        }
    }
}

/// File-based storage (single file per index)
pub struct FileStorage {
    file: Option<File>,
    file_path: String,
    page_size: usize,
    next_page_id: u64,
    file_size: u64,
    header: Option<BTreeHeader>,
}

impl FileStorage {
    /// Open or create a file storage
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref();
        let parent = path.parent();
        if let Some(p) = parent {
            create_dir_all(p)?;
        }

        let file_exists = path.exists();

        let mut opts = OpenOptions::new();
        opts.read(true).write(true).create(true);
        if !file_exists {
            opts.truncate(true);
        }
        let mut file = opts.open(path)?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();
        let page_size = DEFAULT_PAGE_SIZE;

        // Read header from page 0 if file exists and has data
        let header = if file_exists && file_size >= page_size as u64 {
            let mut header_bytes = vec![0u8; page_size];
            use std::io::Seek;
            use std::io::SeekFrom;
            file.seek(SeekFrom::Start(0))?;
            use std::io::Read;
            if file.read_exact(&mut header_bytes).is_ok() {
                bincode::deserialize(&header_bytes).ok()
            } else {
                None
            }
        } else {
            None
        };

        // Determine page count from file size
        let next_page_id = if file_size > 0 {
            file_size / page_size as u64
        } else {
            0
        };

        Ok(Self {
            file: Some(file),
            file_path: path.to_string_lossy().to_string(),
            page_size,
            next_page_id,
            file_size,
            header,
        })
    }

    /// Get header
    pub fn header(&self) -> Option<&BTreeHeader> {
        self.header.as_ref()
    }

    /// Set header and write to disk (page 0)
    pub fn set_header(&mut self, header: BTreeHeader) {
        self.header = Some(header.clone());
        // Write header to page 0
        if let Some(ref mut f) = self.file {
            use std::io::{Seek, SeekFrom, Write};
            let data = bincode::serialize(&header).unwrap_or_default();
            let _ = f.seek(SeekFrom::Start(0));
            let _ = f.write_all(&data);
        }
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

    /// Write header to page 0
    pub fn write_header(&mut self, header: &BTreeHeader) {
        self.header = Some(header.clone());
        if let Some(ref mut f) = self.file {
            use std::io::{Seek, SeekFrom, Write};
            let data = bincode::serialize(header).unwrap_or_default();
            let _ = f.seek(SeekFrom::Start(0));
            let _ = f.write_all(&data);
            self.file_size = self.file_size.max(data.len() as u64);
        }
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
            header: None,
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
