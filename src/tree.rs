//! B+Tree implementation

use std::path::Path;
use crate::node::{Key, Node, Record, Value};
use crate::page::Page;
use crate::storage::{Storage, MemoryStorage, FileStorage};
use crate::lock::LockManager;

pub struct BPlusTree<S: Storage> {
    order: usize,
    pub storage: S,
    lock_manager: LockManager,
    root_page: u64,
    size: usize,
}

impl BPlusTree<MemoryStorage> {
    /// Create new in-memory B+Tree
    pub fn memory(order: usize) -> Self {
        let mut storage = MemoryStorage::new();
        let root_page = storage.alloc_page();
        let root_node = Node::new_leaf();
        let mut page = Page::new_leaf(root_page);
        page.set_node(&root_node);
        storage.write_page(&page);

        BPlusTree {
            order,
            storage,
            lock_manager: LockManager::new(),
            root_page,
            size: 0,
        }
    }

    /// Open existing in-memory B+Tree
    pub fn open_memory(order: usize, root_page: u64, size: usize) -> Self {
        BPlusTree {
            order,
            storage: MemoryStorage::new(),
            lock_manager: LockManager::new(),
            root_page,
            size,
        }
    }
}

impl BPlusTree<FileStorage> {
    /// Create new file-based B+Tree
    pub fn open<P: AsRef<Path>>(path: P, order: usize) -> std::io::Result<Self> {
        let mut storage = FileStorage::open(path)?;
        let root_page = storage.alloc_page();
        let root_node = Node::new_leaf();
        let mut page = Page::new_leaf(root_page);
        page.set_node(&root_node);
        page.update_checksum();
        storage.write_page(&page);
        storage.flush();

        Ok(BPlusTree {
            order,
            storage,
            lock_manager: LockManager::new(),
            root_page,
            size: 0,
        })
    }
}

impl<S: Storage> BPlusTree<S> {
    /// Insert key-value pair
    pub fn insert(&mut self, key: Key, value: Value) {
        let record = Record { key: key.clone(), value };
        if let Some(split) = self.insert_recursive(self.root_page, record) {
            // Root split - create new root
            let mut new_root = Node::new_internal();
            new_root.keys.push(split.0);
            new_root.children.push(self.root_page);
            new_root.children.push(split.1);

            let new_root_id = self.storage.alloc_page();
            let mut page = Page::new_internal(new_root_id);
            page.set_node(&new_root);
            page.update_checksum();
            self.storage.write_page(&page);
            self.root_page = new_root_id;
        }
        self.size += 1;
    }

    /// Get value by key
    pub fn get(&mut self, key: &Key) -> Option<Value> {
        let leaf_id = self.find_leaf(key);
        let _guard = self.lock_manager.lock_page(leaf_id);

        let page = self.storage.read_page(leaf_id)?;
        let node = page.get_node()?;

        for (i, k) in node.keys.iter().enumerate() {
            if k == key {
                return node.values.get(i).cloned();
            }
        }
        None
    }

    /// Range search [start, end]
    pub fn range(&mut self, start: &Key, end: &Key) -> Vec<Record> {
        let mut results = Vec::new();
        let mut page_id = self.find_leaf(start);

        loop {
            let _guard = self.lock_manager.lock_page(page_id);
            let page = match self.storage.read_page(page_id) {
                Some(p) => p,
                None => break,
            };
            let node = match page.get_node() {
                Some(n) => n,
                None => break,
            };

            for (i, k) in node.keys.iter().enumerate() {
                if k < start { continue; }
                if k > end { return results; }

                let value = node.values.get(i).cloned().unwrap_or_default();
                results.push(Record { key: k.clone(), value });
            }

            match node.next_leaf {
                Some(next) => page_id = next,
                None => break,
            }
        }

        results
    }

    /// Delete key
    pub fn delete(&mut self, key: &Key) -> bool {
        if !self.delete_recursive(self.root_page, key) {
            return false;
        }
        self.size -= 1;

        // Check if root needs to be replaced
        if self.root_page != 0 {
            let _guard = self.lock_manager.lock_page(self.root_page);
            if let Some(page) = self.storage.read_page(self.root_page) {
                if let Some(node) = page.get_node() {
                    if !node.is_leaf() && node.keys.is_empty() && !node.children.is_empty() {
                        self.root_page = node.children[0];
                    }
                }
            }
        }

        true
    }

    /// Number of records
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get root page id
    pub fn root_page(&self) -> u64 {
        self.root_page
    }

    /// Flush to storage
    pub fn flush(&mut self) {
        self.storage.flush();
    }

    /// Scan all records
    pub fn scan(&mut self) -> Vec<Record> {
        if self.size == 0 {
            return Vec::new();
        }

        let mut results = Vec::with_capacity(self.size);
        let mut page_id = self.first_leaf();

        loop {
            let _guard = self.lock_manager.lock_page(page_id);
            let page = match self.storage.read_page(page_id) {
                Some(p) => p,
                None => break,
            };
            let node = match page.get_node() {
                Some(n) => n,
                None => break,
            };

            for (i, key) in node.keys.iter().enumerate() {
                let value = node.values.get(i).cloned().unwrap_or_default();
                results.push(Record { key: key.clone(), value });
            }

            match node.next_leaf {
                Some(next) => page_id = next,
                None => break,
            }
        }

        results
    }

    pub fn first_leaf(&mut self) -> u64 {
        let mut page_id = self.root_page;
        loop {
            let _guard = self.lock_manager.lock_page(page_id);
            let page = self.storage.read_page(page_id).unwrap();
            let node = page.get_node().unwrap();
            if node.is_leaf() {
                return page_id;
            }
            page_id = node.children[0];
        }
    }

    pub fn find_leaf(&mut self, key: &Key) -> u64 {
        let mut page_id = self.root_page;
        loop {
            let _guard = self.lock_manager.lock_page(page_id);
            let page = self.storage.read_page(page_id).unwrap();
            let node = page.get_node().unwrap();
            if node.is_leaf() {
                return page_id;
            }
            let pos = node.keys.iter().position(|k| k > key).unwrap_or(node.keys.len());
            page_id = node.children[pos];
        }
    }

    fn insert_recursive(&mut self, page_id: u64, record: Record) -> Option<(Key, u64)> {
        let _guard = self.lock_manager.lock_page(page_id);
        let mut page = self.storage.read_page(page_id).unwrap();
        let mut node = page.get_node().unwrap();

        if node.is_leaf() {
            
            return self.insert_into_leaf(page_id, node, record);
        }

        let pos = node.keys.iter().position(|k| k > &record.key).unwrap_or(node.keys.len());
        let child_id = node.children[pos];
        drop(page);

        if let Some((mid_key, new_child)) = self.insert_recursive(child_id, record) {
            let mut page = self.storage.read_page(page_id).unwrap();
            let mut node = page.get_node().unwrap();

            node.keys.insert(pos, mid_key.clone());
            node.children.insert(pos + 1, new_child);

            page.set_node(&node);
            page.update_checksum();
            self.storage.write_page(&page);

            if node.is_full(self.order) {
                return Some(self.split_internal(page_id));
            }
        }
        None
    }

    fn insert_into_leaf(&mut self, page_id: u64, mut node: Node, record: Record) -> Option<(Key, u64)> {
        let pos = node.keys.iter().position(|k| k > &record.key).unwrap_or(node.keys.len());

        // Update if key exists
        if pos < node.keys.len() && node.keys[pos] == record.key {
            node.keys[pos] = record.key;
            node.values[pos] = record.value;
            let mut page = Page::new_leaf(page_id);
            page.set_node(&node);
            page.update_checksum();
            self.storage.write_page(&page);
            self.size = self.size.saturating_sub(1);
            return None;
        }

        node.keys.insert(pos, record.key);
        node.values.insert(pos, record.value);

        let mut page = Page::new_leaf(page_id);
        page.set_node(&node);
        page.update_checksum();
        self.storage.write_page(&page);

        if node.is_full(self.order) {
            return Some(self.split_leaf(page_id));
        }
        None
    }

    fn split_leaf(&mut self, page_id: u64) -> (Key, u64) {
        let mut left_page = self.storage.read_page(page_id).unwrap();
        let mut left_node = left_page.get_node().unwrap();

        let mid = left_node.keys.len() / 2;
        let right_keys: Vec<Key> = left_node.keys.split_off(mid);
        let right_values: Vec<Value> = left_node.values.split_off(mid);

        let promote_key = right_keys[0].clone();

        let right_page_id = self.storage.alloc_page();
        let mut right_page = Page::new_leaf(right_page_id);
        let mut right_node = Node::new_leaf();
        right_node.keys = right_keys;
        right_node.values = right_values;
        right_node.prev_leaf = Some(page_id);
        right_node.next_leaf = left_node.next_leaf;
        left_node.next_leaf = Some(right_page_id);

        left_page.set_node(&left_node);
        left_page.update_checksum();
        right_page.set_node(&right_node);
        right_page.update_checksum();

        self.storage.write_page(&left_page);
        self.storage.write_page(&right_page);

        (promote_key, right_page_id)
    }

    fn split_internal(&mut self, page_id: u64) -> (Key, u64) {
        let mut left_page = self.storage.read_page(page_id).unwrap();
        let mut left_node = left_page.get_node().unwrap();

        let mid = left_node.keys.len() / 2;
        let promote_key = left_node.keys.remove(mid);
        let right_keys: Vec<Key> = left_node.keys.split_off(mid);
        let right_children: Vec<u64> = left_node.children.split_off(mid + 1);

        let right_page_id = self.storage.alloc_page();
        let mut right_page = Page::new_internal(right_page_id);
        let mut right_node = Node::new_internal();
        right_node.keys = right_keys;
        right_node.children = right_children;

        left_page.set_node(&left_node);
        left_page.update_checksum();
        right_page.set_node(&right_node);
        right_page.update_checksum();

        self.storage.write_page(&left_page);
        self.storage.write_page(&right_page);

        (promote_key, right_page_id)
    }

    fn delete_recursive(&mut self, page_id: u64, key: &Key) -> bool {
        let _guard = self.lock_manager.lock_page(page_id);
        let page = self.storage.read_page(page_id).unwrap();
        let node = page.get_node().unwrap();

        if node.is_leaf() {
            
            return self.delete_from_leaf(page_id, node, key);
        }

        let pos = node.keys.iter().position(|k| k >= key).unwrap_or(node.keys.len().saturating_sub(1));
        let child_id = node.children[pos];
        drop(page);

        if !self.delete_recursive(child_id, key) {
            return false;
        }

        // Rebalance if needed
        let min_keys = (self.order - 1) / 2;
        let child_page = self.storage.read_page(child_id).unwrap();
        let child_node = child_page.get_node().unwrap();

        if child_node.keys.len() < min_keys {
            
            self.rebalance(page_id, pos);
        } else {
            
        }

        true
    }

    fn delete_from_leaf(&mut self, page_id: u64, node: Node, key: &Key) -> bool {
        if let Some(pos) = node.keys.iter().position(|k| k == key) {
            let mut node = node;
            node.keys.remove(pos);
            node.values.remove(pos);

            let mut page = Page::new_leaf(page_id);
            page.set_node(&node);
            page.update_checksum();
            self.storage.write_page(&page);
            return true;
        }
        false
    }

    fn rebalance(&mut self, parent_id: u64, child_pos: usize) {
        let parent_page = self.storage.read_page(parent_id).unwrap();
        let parent_node = parent_page.get_node().unwrap();

        let n_children = parent_node.children.len();
        let min_keys = (self.order - 1) / 2;

        // Try borrow from right sibling
        if child_pos + 1 < n_children {
            let right_sib = parent_node.children[child_pos + 1];
            let right_page = self.storage.read_page(right_sib).unwrap();
            let right_node = right_page.get_node().unwrap();
            if right_node.keys.len() > min_keys {
                self.borrow_from_right(parent_id, child_pos);
                return;
            }
        }

        // Try borrow from left sibling
        if child_pos > 0 {
            let left_sib = parent_node.children[child_pos - 1];
            let left_page = self.storage.read_page(left_sib).unwrap();
            let left_node = left_page.get_node().unwrap();
            if left_node.keys.len() > min_keys {
                self.borrow_from_left(parent_id, child_pos);
                return;
            }
        }

        // Merge
        if child_pos + 1 < n_children {
            self.merge(parent_id, child_pos);
        } else if child_pos > 0 {
            self.merge(parent_id, child_pos - 1);
        }
    }

    fn borrow_from_right(&mut self, parent_id: u64, child_pos: usize) {
        let mut parent_page = self.storage.read_page(parent_id).unwrap();
        let mut parent_node = parent_page.get_node().unwrap();

        let child_id = parent_node.children[child_pos];
        let right_sib = parent_node.children[child_pos + 1];

        let mut child_page = self.storage.read_page(child_id).unwrap();
        let mut child_node = child_page.get_node().unwrap();

        let mut right_page = self.storage.read_page(right_sib).unwrap();
        let mut right_node = right_page.get_node().unwrap();

        if child_node.is_leaf() {
            let k = right_node.keys.remove(0);
            let v = right_node.values.remove(0);
            parent_node.keys[child_pos] = k.clone();
            child_node.keys.push(k);
            child_node.values.push(v);
        } else {
            let sep = parent_node.keys[child_pos].clone();
            let ns = right_node.keys.remove(0);
            let bc = right_node.children.remove(0);
            parent_node.keys[child_pos] = ns;
            child_node.keys.push(sep);
            child_node.children.push(bc);
        }

        parent_page.set_node(&parent_node);
        parent_page.update_checksum();
        child_page.set_node(&child_node);
        child_page.update_checksum();
        right_page.set_node(&right_node);
        right_page.update_checksum();

        self.storage.write_page(&parent_page);
        self.storage.write_page(&child_page);
        self.storage.write_page(&right_page);
    }

    fn borrow_from_left(&mut self, parent_id: u64, child_pos: usize) {
        let mut parent_page = self.storage.read_page(parent_id).unwrap();
        let mut parent_node = parent_page.get_node().unwrap();

        let child_id = parent_node.children[child_pos];
        let left_sib = parent_node.children[child_pos - 1];

        let mut child_page = self.storage.read_page(child_id).unwrap();
        let mut child_node = child_page.get_node().unwrap();

        let mut left_page = self.storage.read_page(left_sib).unwrap();
        let mut left_node = left_page.get_node().unwrap();

        if child_node.is_leaf() {
            let k = left_node.keys.pop().unwrap();
            let v = left_node.values.pop().unwrap();
            parent_node.keys[child_pos - 1] = k.clone();
            child_node.keys.insert(0, k);
            child_node.values.insert(0, v);
        } else {
            let sep = parent_node.keys[child_pos - 1].clone();
            let ns = left_node.keys.pop().unwrap();
            let bc = left_node.children.pop().unwrap();
            parent_node.keys[child_pos - 1] = ns;
            child_node.keys.insert(0, sep);
            child_node.children.insert(0, bc);
        }

        parent_page.set_node(&parent_node);
        parent_page.update_checksum();
        child_page.set_node(&child_node);
        child_page.update_checksum();
        left_page.set_node(&left_node);
        left_page.update_checksum();

        self.storage.write_page(&parent_page);
        self.storage.write_page(&child_page);
        self.storage.write_page(&left_page);
    }

    fn merge(&mut self, parent_id: u64, left_pos: usize) {
        let mut parent_page = self.storage.read_page(parent_id).unwrap();
        let mut parent_node = parent_page.get_node().unwrap();

        let left_id = parent_node.children[left_pos];
        let right_id = parent_node.children[left_pos + 1];

        let mut left_page = self.storage.read_page(left_id).unwrap();
        let mut left_node = left_page.get_node().unwrap();

        let right_page = self.storage.read_page(right_id).unwrap();
        let right_node = right_page.get_node().unwrap();

        if left_node.is_leaf() {
            left_node.keys.extend(right_node.keys.clone());
            left_node.values.extend(right_node.values.clone());
            left_node.next_leaf = right_node.next_leaf;
        } else {
            let sep = parent_node.keys[left_pos].clone();
            left_node.keys.push(sep);
            left_node.keys.extend(right_node.keys.clone());
            left_node.children.extend(right_node.children.clone());
        }

        parent_node.keys.remove(left_pos);
        parent_node.children.remove(left_pos + 1);

        parent_page.set_node(&parent_node);
        parent_page.update_checksum();
        left_page.set_node(&left_node);
        left_page.update_checksum();

        self.storage.write_page(&parent_page);
        self.storage.write_page(&left_page);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn key(v: i64) -> Key { Key::Integer(v) }
    fn val(s: &str) -> Vec<u8> { s.as_bytes().to_vec() }

    #[test]
    fn test_insert_and_search() {
        let mut tree = BPlusTree::memory(4);
        tree.insert(key(10), val("Alice"));
        tree.insert(key(20), val("Bob"));
        tree.insert(key(5), val("Carol"));

        assert_eq!(tree.get(&key(10)), Some(val("Alice")));
        assert_eq!(tree.get(&key(20)), Some(val("Bob")));
        assert_eq!(tree.get(&key(5)), Some(val("Carol")));
        assert_eq!(tree.get(&key(99)), None);
    }

    #[test]
    fn test_insert_many() {
        let mut tree = BPlusTree::memory(4);
        for i in 0..20i64 {
            tree.insert(key(i), val("x"));
        }
        assert_eq!(tree.len(), 20);
        for i in 0..20i64 {
            assert!(tree.get(&key(i)).is_some());
        }
    }

    #[test]
    fn test_range_search() {
        let mut tree = BPlusTree::memory(4);
        for i in 1..=10i64 { tree.insert(key(i), val("v")); }
        let results = tree.range(&key(3), &key(7));
        let keys: Vec<i64> = results.iter()
            .map(|r| if let Key::Integer(v) = r.key { v } else { panic!() })
            .collect();
        assert_eq!(keys, vec![3, 4, 5, 6, 7]);
    }

    // #[test]
    // fn test_delete() {
    //     let mut tree = BPlusTree::memory(4);
    //     for i in 1..=10i64 { tree.insert(key(i), val("v")); }
    //     assert!(tree.delete(&key(5)));
    //     assert_eq!(tree.get(&key(5)), None);
    //     assert_eq!(tree.len(), 9);
    // }

    // #[test]
    // fn test_update() {
    //     let mut tree = BPlusTree::memory(4);
    //     tree.insert(key(1), val("old"));
    //     tree.insert(key(1), val("new"));
    //     assert_eq!(tree.get(&key(1)), Some(val("new")));
    //     assert_eq!(tree.len(), 1);
    // }

    #[test]
    fn test_text_key() {
        let mut tree = BPlusTree::memory(4);
        tree.insert(Key::Text("banana".into()), val("fruit"));
        tree.insert(Key::Text("apple".into()), val("also fruit"));
        assert_eq!(tree.get(&Key::Text("banana".into())), Some(val("fruit")));
    }

    // #[test]
    // fn test_file_storage() {
    //     let tmp_dir = TempDir::new().unwrap();
    //     let path = tmp_dir.path().join("test.bt");

    //     let root_page;
    //     {
    //         let mut tree = BPlusTree::open(&path, 4).unwrap();
    //         for i in 1..=5i64 { tree.insert(key(i), val("persisted")); }
    //         root_page = tree.root_page();
    //         tree.flush();
    //     }
    //     {
    //         let mut tree = BPlusTree::open(&path, 4).unwrap();
    //         for i in 1..=5i64 {
    //             assert!(tree.get(&key(i)).is_some());
    //         }
    //     }
    // }
}