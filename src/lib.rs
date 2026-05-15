//! btree6 - B+Tree implementation in Rust
//!
//! A high-performance, concurrent B+Tree with PostgreSQL-style design.
//!
//! # Quick Start
//!
//! ```rust
//! use btree6::{BPlusTree, Key};
//!
//! let mut tree = BPlusTree::memory(4);
//! tree.insert(Key::Integer(42), b"hello".to_vec());
//! assert_eq!(tree.get(&Key::Integer(42)), Some(b"hello".to_vec()));
//! ```

pub mod node;
pub mod page;
pub mod tree;
pub mod storage;
pub mod lock;
pub mod cursor;

pub use tree::BPlusTree;
pub use node::{Key, Value, Record};
pub use storage::{Storage, FileStorage, MemoryStorage};
pub use lock::LockManager;
pub use cursor::Cursor;

pub const DEFAULT_PAGE_SIZE: usize = 8192;
pub const DEFAULT_ORDER: usize = 64;