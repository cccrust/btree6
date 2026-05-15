//! B+Tree Node definition

use serde::{Deserialize, Serialize};

/// Key type supporting integers and strings
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Key {
    Integer(i64),
    Text(String),
    Bytes(Vec<u8>),
}

impl Key {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Key::Integer(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Key::Text(s) => Some(s),
            _ => None,
        }
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Integer(v) => write!(f, "{}", v),
            Key::Text(s) => write!(f, "{}", s),
            Key::Bytes(b) => write!(f, "b\"{}\"", String::from_utf8_lossy(b)),
        }
    }
}

impl From<i64> for Key {
    fn from(v: i64) -> Self {
        Key::Integer(v)
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Key::Text(s)
    }
}

impl From<&str> for Key {
    fn from(s: &str) -> Self {
        Key::Text(s.to_string())
    }
}

/// Value type - raw bytes
pub type Value = Vec<u8>;

/// Record: key-value pair
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub key: Key,
    pub value: Value,
}

impl Record {
    pub fn new(key: Key, value: Value) -> Self {
        Self { key, value }
    }
}

/// Node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Internal,
    Leaf,
}

/// B+Tree node structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub node_type: NodeType,
    pub keys: Vec<Key>,
    pub children: Vec<u64>, // page ids
    pub values: Vec<Value>, // only for leaf
    pub next_leaf: Option<u64>,
    pub prev_leaf: Option<u64>,
}

impl Node {
    pub fn new_leaf() -> Self {
        Node {
            node_type: NodeType::Leaf,
            keys: Vec::new(),
            children: Vec::new(),
            values: Vec::new(),
            next_leaf: None,
            prev_leaf: None,
        }
    }

    pub fn new_internal() -> Self {
        Node {
            node_type: NodeType::Internal,
            keys: Vec::new(),
            children: Vec::new(),
            values: Vec::new(),
            next_leaf: None,
            prev_leaf: None,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.node_type == NodeType::Leaf
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Check if node needs split (keys >= order - 1)
    pub fn is_full(&self, order: usize) -> bool {
        self.keys.len() >= order - 1
    }

    /// Number of key-value pairs
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_minimal(&self, order: usize) -> bool {
        self.keys.len() < (order - 1) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_ordering() {
        let k1 = Key::Integer(1);
        let k2 = Key::Integer(10);
        assert!(k1 < k2);

        let t1 = Key::Text("apple".to_string());
        let t2 = Key::Text("banana".to_string());
        assert!(t1 < t2);
    }

    #[test]
    fn test_node_new() {
        let leaf = Node::new_leaf();
        assert!(leaf.is_leaf());
        assert!(leaf.is_empty());

        let internal = Node::new_internal();
        assert!(!internal.is_leaf());
    }
}
