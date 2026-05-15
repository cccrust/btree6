//! Cursor for B+Tree iteration

use crate::node::{Key, Record};
use crate::tree::BPlusTree;
use crate::storage::Storage;

pub struct Cursor<'a, S: Storage> {
    tree: &'a mut BPlusTree<S>,
    current_page: Option<u64>,
    index_in_page: usize,
    done: bool,
}

impl<'a, S: Storage> Cursor<'a, S> {
    pub fn new(tree: &'a mut BPlusTree<S>) -> Self {
        let is_empty = tree.is_empty();
        let current_page = if is_empty {
            None
        } else {
            Some(tree.first_leaf())
        };

        Cursor {
            tree,
            current_page,
            index_in_page: 0,
            done: is_empty,
        }
    }

    #[allow(dead_code)]
    pub fn from_key(tree: &'a mut BPlusTree<S>, key: &Key) -> Self {
        let is_empty = tree.is_empty();
        let current_page = if is_empty {
            None
        } else {
            let leaf = tree.find_leaf(key);
            Some(leaf)
        };

        Cursor {
            tree,
            current_page,
            index_in_page: 0,
            done: is_empty,
        }
    }
}

impl<'a, S: Storage> Iterator for Cursor<'a, S> {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        loop {
            let page_id = match self.current_page {
                Some(id) => id,
                None => {
                    self.done = true;
                    return None;
                }
            };

            let page = self.tree.storage.read_page(page_id)?;
            let node = page.get_node()?;

            if self.index_in_page >= node.keys.len() {
                // Move to next leaf
                self.current_page = node.next_leaf;
                self.index_in_page = 0;
                if self.current_page.is_none() {
                    self.done = true;
                    return None;
                }
                continue;
            }

            let key = node.keys[self.index_in_page].clone();
            let value = node.values.get(self.index_in_page).cloned().unwrap_or_default();
            self.index_in_page += 1;

            return Some(Record { key, value });
        }
    }
}

impl<'a, S: Storage> IntoIterator for &'a mut BPlusTree<S> {
    type Item = Record;
    type IntoIter = Cursor<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        Cursor::new(self)
    }
}