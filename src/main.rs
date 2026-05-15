#![allow(unused)]

//! btree6 command-line tool

use btree6::{BPlusTree, Key};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("btree6 - B+Tree implementation");
        println!("Usage: btree6 <command> [args]");
        println!();
        println!("Commands:");
        println!("  test        Run basic tests");
        println!("  insert <key> <value>  Insert key-value");
        println!("  get <key>   Get value by key");
        println!("  range <start> <end>   Range query");
        println!("  delete <key>          Delete key");
        println!("  scan        Scan all records");
        return;
    }

    match args[1].as_str() {
        "test" => run_tests(),
        "insert" => {
            if args.len() < 4 {
                eprintln!("Usage: btree6 insert <key> <value>");
                return;
            }
            let key: i64 = args[2].parse().expect("Invalid key");
            let value = args[3].as_bytes().to_vec();
            let mut tree = BPlusTree::memory(4);
            tree.insert(Key::Integer(key), value);
            println!("Inserted key {}", key);
        }
        "get" => {
            if args.len() < 3 {
                eprintln!("Usage: btree6 get <key>");
                return;
            }
            let key: i64 = args[2].parse().expect("Invalid key");
            let mut tree = BPlusTree::memory(4);
            tree.insert(Key::Integer(1), b"one".to_vec());
            tree.insert(Key::Integer(2), b"two".to_vec());
            if let Some(val) = tree.get(&Key::Integer(key)) {
                println!("Value: {}", String::from_utf8_lossy(&val));
            } else {
                println!("Key not found");
            }
        }
        "range" => {
            let mut tree = BPlusTree::memory(4);
            for i in 1..=10i64 {
                tree.insert(Key::Integer(i), format!("v{}", i).as_bytes().to_vec());
            }
            let results = tree.range(&Key::Integer(3), &Key::Integer(7));
            for r in results {
                println!("{}: {}", r.key, String::from_utf8_lossy(&r.value));
            }
        }
        "scan" => {
            let mut tree = BPlusTree::memory(4);
            tree.insert(Key::Integer(3), b"c".to_vec());
            tree.insert(Key::Integer(1), b"a".to_vec());
            tree.insert(Key::Integer(2), b"b".to_vec());
            for r in &mut tree {
                println!("{}: {}", r.key, String::from_utf8_lossy(&r.value));
            }
        }
        "delete" => {
            if args.len() < 3 {
                eprintln!("Usage: btree6 delete <key>");
                return;
            }
            let key: i64 = args[2].parse().expect("Invalid key");
            let mut tree = BPlusTree::memory(4);
            tree.insert(Key::Integer(key), b"value".to_vec());
            if tree.delete(&Key::Integer(key)) {
                println!("Deleted key {}", key);
            } else {
                println!("Key not found");
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }
}

fn run_tests() {
    use btree6::{Key, Value};

    let mut tree = BPlusTree::memory(4);

    // Test insert and search
    tree.insert(Key::Integer(10), b"Alice".to_vec());
    tree.insert(Key::Integer(20), b"Bob".to_vec());
    tree.insert(Key::Integer(5), b"Carol".to_vec());

    assert_eq!(tree.get(&Key::Integer(10)), Some(b"Alice".to_vec()));
    assert_eq!(tree.get(&Key::Integer(20)), Some(b"Bob".to_vec()));
    assert_eq!(tree.get(&Key::Integer(5)), Some(b"Carol".to_vec()));
    assert_eq!(tree.get(&Key::Integer(99)), None);

    // Test many inserts (0-19 = 20 keys)
    // Keys 10, 20, 5 already exist, so they update
    for i in 0..20i64 {
        tree.insert(Key::Integer(i), b"x".to_vec());
    }
    // Total: 3 (initial) + 20 - 2 (10 and 20 overlap) = 21
    // Actually: 5 is also in 0-19, so 3 overlaps: 3 + 20 - 3 = 20...
    // Wait: 5, 10, 20 are in 0-19, so 3 overlaps. But we inserted 0-19 which includes 5,10
    // Let's just check that get works:
    assert_eq!(tree.len(), 21);
    for i in 0..20i64 {
        assert!(tree.get(&Key::Integer(i)).is_some());
    }

    // Test range search
    let mut tree2 = BPlusTree::memory(4);
    for i in 1..=10i64 {
        tree2.insert(Key::Integer(i), b"v".to_vec());
    }
    let results = tree2.range(&Key::Integer(3), &Key::Integer(7));
    let keys: Vec<i64> = results
        .iter()
        .map(|r| {
            if let Key::Integer(v) = r.key {
                v
            } else {
                panic!()
            }
        })
        .collect();
    assert_eq!(keys, vec![3, 4, 5, 6, 7]);

    // Test delete
    let mut tree3 = BPlusTree::memory(4);
    for i in 1..=10i64 {
        tree3.insert(Key::Integer(i), b"v".to_vec());
    }
    assert!(tree3.delete(&Key::Integer(5)));
    assert_eq!(tree3.get(&Key::Integer(5)), None);
    assert_eq!(tree3.len(), 9);

    // Test update
    let mut tree4 = BPlusTree::memory(4);
    tree4.insert(Key::Integer(1), b"old".to_vec());
    tree4.insert(Key::Integer(1), b"new".to_vec());
    assert_eq!(tree4.get(&Key::Integer(1)), Some(b"new".to_vec()));
    assert_eq!(tree4.len(), 1);

    println!("All tests passed!");
}
