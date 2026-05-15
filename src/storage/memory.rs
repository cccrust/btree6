//! In-memory storage

use crate::DEFAULT_PAGE_SIZE;
use crate::page::Page;
use crate::storage::BTreeHeader;
use std::collections::HashMap;

/// In-memory storage
pub struct MemoryStorage {
    pub pages: HashMap<u64, Page>,
    pub next_page_id: u64,
    pub page_size: usize,
    pub header: Option<BTreeHeader>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            pages: HashMap::new(),
            next_page_id: 0,
            page_size: DEFAULT_PAGE_SIZE,
            header: None,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pages: HashMap::with_capacity(capacity),
            next_page_id: 0,
            page_size: DEFAULT_PAGE_SIZE,
            header: None,
        }
    }

    pub fn clear(&mut self) {
        self.pages.clear();
        self.next_page_id = 0;
    }

    pub fn set_header(&mut self, header: BTreeHeader) {
        self.header = Some(header);
    }

    pub fn header(&self) -> Option<&BTreeHeader> {
        self.header.as_ref()
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemoryStorage {
    fn clone(&self) -> Self {
        Self {
            pages: self.pages.clone(),
            next_page_id: self.next_page_id,
            page_size: self.page_size,
            header: self.header.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;

    #[test]
    fn test_memory_storage() {
        let mut storage = MemoryStorage::new();
        let page_id = storage.alloc_page();
        assert_eq!(page_id, 0);

        let page = Page::new_leaf(page_id);
        storage.write_page(&page);

        let read = storage.read_page(page_id);
        assert!(read.is_some());
    }

    #[test]
    fn test_alloc_multiple() {
        let mut storage = MemoryStorage::new();
        for i in 0..10u64 {
            assert_eq!(storage.alloc_page(), i);
        }
    }
}
