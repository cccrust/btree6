//! Storage backends

mod file;
mod memory;

pub use file::FileStorage;
pub use memory::MemoryStorage;

use crate::page::Page;
use std::io::{Read, Seek, SeekFrom, Write};

/// Storage trait
pub trait Storage: Send + Sync {
    fn read_page(&mut self, page_id: u64) -> Option<Page>;
    fn write_page(&mut self, page: &Page);
    fn alloc_page(&mut self) -> u64;
    fn page_count(&self) -> u64;
    fn flush(&mut self);
    fn close(&mut self);
}

/// In-memory storage for testing
impl Storage for MemoryStorage {
    fn read_page(&mut self, page_id: u64) -> Option<Page> {
        self.pages.get(&page_id).cloned()
    }

    fn write_page(&mut self, page: &Page) {
        self.pages.insert(page.header.page_id, page.clone());
    }

    fn alloc_page(&mut self) -> u64 {
        let id = self.next_page_id;
        self.next_page_id += 1;
        id
    }

    fn page_count(&self) -> u64 {
        self.next_page_id
    }

    fn flush(&mut self) {}

    fn close(&mut self) {
        self.pages.clear();
        self.next_page_id = 0;
    }
}

/// File-based storage
impl Storage for FileStorage {
    fn read_page(&mut self, page_id: u64) -> Option<Page> {
        let offset = page_id * self.page_size() as u64;
        if offset >= self.file_size() {
            return None;
        }

        let mut buffer = vec![0u8; self.page_size()];
        if let Some(ref mut f) = *self.file_mut() {
            if f.seek(SeekFrom::Start(offset)).is_err() {
                return None;
            }
            if f.read(&mut buffer).is_err() {
                return None;
            }
            bincode::deserialize(&buffer).ok()
        } else {
            None
        }
    }

    fn write_page(&mut self, page: &Page) {
        let data = bincode::serialize(page).unwrap_or_default();
        let page_id = page.header.page_id;
        let offset = page_id * self.page_size() as u64;

        // Extend file if needed
        if offset + data.len() as u64 > self.file_size() {
            self.set_file_size(offset + data.len() as u64);
        }

        if let Some(ref mut f) = *self.file_mut() {
            let _ = f.seek(SeekFrom::Start(offset));
            let _ = f.write_all(&data);
        }
    }

    fn alloc_page(&mut self) -> u64 {
        let id = self.next_page_id();
        self.set_next_page_id(id + 1);
        id
    }

    fn page_count(&self) -> u64 {
        self.next_page_id()
    }

    fn flush(&mut self) {
        if let Some(ref mut f) = *self.file_mut() {
            let _ = f.flush();
        }
    }

    fn close(&mut self) {
        if let Some(ref mut f) = *self.file_mut() {
            let _ = f.flush();
        }
        *self.file_mut() = None;
    }
}
