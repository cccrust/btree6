//! Page structure - PostgreSQL style

use serde::{Deserialize, Serialize};
use crate::node::Node;
use crate::DEFAULT_PAGE_SIZE;

const PAGE_MAGIC: u32 = 0x42545636; // "BTV6"

/// Page header (32 bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageHeader {
    pub magic: u32,          // 4 bytes: magic number
    pub page_id: u64,       // 8 bytes: page number
    pub page_size: u32,     // 4 bytes: page size
    pub node_type: u8,      // 1 byte: node type (0=internal, 1=leaf)
    pub is_valid: u8,      // 1 byte: valid flag
    pub free_space: u16,    // 2 bytes: free space offset
    pub item_count: u16,    // 2 bytes: number of items
    pub lsn: u64,           // 8 bytes: WAL LSN (optional)
    pub checksum: u32,      // 4 bytes: CRC32
}

impl PageHeader {
    pub fn new(page_id: u64, node_type: u8) -> Self {
        let page_size = DEFAULT_PAGE_SIZE as u32;
        let free_space = (std::mem::size_of::<PageHeader>() + std::mem::size_of::<ItemId>()) as u16;
        PageHeader {
            magic: PAGE_MAGIC,
            page_id,
            page_size,
            node_type,
            is_valid: 1,
            free_space,
            item_count: 0,
            lsn: 0,
            checksum: 0,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.node_type == 1
    }
}

/// Item ID - pointer to actual data in page
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ItemId {
    pub offset: u16,   // offset from page start
    pub length: u16,   // data length
}

impl ItemId {
    pub fn new(offset: u16, length: u16) -> Self {
        Self { offset, length }
    }
}

/// Page structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub header: PageHeader,
    pub items: Vec<ItemId>,
    pub data: Vec<u8>,  // serialized node
}

impl Page {
    pub fn new_leaf(page_id: u64) -> Self {
        Page {
            header: PageHeader::new(page_id, 1),
            items: Vec::new(),
            data: Vec::new(),
        }
    }

    pub fn new_internal(page_id: u64) -> Self {
        Page {
            header: PageHeader::new(page_id, 0),
            items: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Serialize node into page data
    pub fn set_node(&mut self, node: &Node) {
        self.data = bincode::serialize(node).unwrap_or_default();
        self.update_free_space();
    }

    /// Deserialize node from page data
    #[allow(dead_code)]
    pub fn get_node(&self) -> Option<Node> {
        bincode::deserialize(&self.data).ok()
    }

    pub fn update_free_space(&mut self) {
        let data_end = std::mem::size_of::<PageHeader>()
            + self.items.len() * std::mem::size_of::<ItemId>()
            + self.data.len();
        self.header.free_space = data_end as u16;
    }

    pub fn update_checksum(&mut self) {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&self.header.magic.to_be_bytes());
        hasher.update(&self.header.page_id.to_be_bytes());
        hasher.update(&self.data);
        self.header.checksum = hasher.finalize();
    }

    pub fn verify_checksum(&self) -> bool {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&self.header.magic.to_be_bytes());
        hasher.update(&self.header.page_id.to_be_bytes());
        hasher.update(&self.data);
        hasher.finalize() == self.header.checksum
    }

    /// Check if page is valid
    pub fn is_valid(&self) -> bool {
        self.header.magic == PAGE_MAGIC && self.header.is_valid == 1
    }

    /// Remaining free space
    pub fn free_space(&self) -> usize {
        (self.header.page_size as usize) - self.header.free_space as usize
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new_leaf(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_header() {
        let header = PageHeader::new(1, 1);
        assert_eq!(header.page_id, 1);
        assert!(header.is_leaf());
    }

    #[test]
    fn test_page_serialize_node() {
        let mut page = Page::new_leaf(0);
        let node = Node::new_leaf();
        page.set_node(&node);
        assert!(!page.data.is_empty());

        let restored = page.get_node().unwrap();
        assert!(restored.is_leaf());
    }

    #[test]
    fn test_checksum() {
        let mut page = Page::new_leaf(1);
        let node = Node::new_leaf();
        page.set_node(&node);
        page.update_checksum();
        assert!(page.verify_checksum());
    }
}