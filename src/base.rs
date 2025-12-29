use std::collections::{HashMap, HashSet};
use std::fs::{self};
use std::path::{Path, PathBuf};

use crate::data::{ObjectType, get_object, hash_object};

fn get_ignored_files() -> HashSet<PathBuf> {
    let mut ignored_files = HashSet::new();

    let ugit_ignore = fs::read_to_string(".ugitignore").expect("Failed to read .ugitignore");

    ugit_ignore
        .lines()
        .filter(|line| !line.is_empty())
        .for_each(|entry| {
            ignored_files.insert(Path::new(".").join(Path::new(entry)));
        });

    ignored_files
}

pub fn write_tree(dir: &Path) -> String {
    let ignored_files = get_ignored_files();
    let mut entries: Vec<(ObjectType, String, String)> = Vec::new();

    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.filter_map(|e| e.ok()) {
            let path = entry.path();

            // skip ignored files/dirs
            if ignored_files.contains(&path) {
                continue;
            }

            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

            if path.is_file() {
                println!("[file] {:?}", path);
                let content = fs::read(&path).expect("Failed to read file");
                let oid = hash_object(&content, ObjectType::Blob);
                entries.push((ObjectType::Blob, oid, file_name));
            } else if path.is_dir() {
                println!("[dir] {:?}", path);
                let oid = write_tree(&path);
                entries.push((ObjectType::Tree, oid, file_name));
            }
        }
    }

    entries.sort_by(|a, b| a.2.cmp(&b.2));

    let mut tree = String::new();
    entries.iter().for_each(|(obj_type, oid, name)| {
        tree.push_str(format!("{} {} {}\n", obj_type.as_str(), oid, name).as_str());
    });
    hash_object(tree.as_bytes(), ObjectType::Tree)
}

fn iter_tree_entries(oid: &str) -> Vec<(ObjectType, String, String)> {
    let tree = get_object(oid, ObjectType::Tree);
    let content = str::from_utf8(&tree).expect("failed to decode file");
    let mut entries: Vec<(ObjectType, String, String)> = Vec::new();
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let obj_type = match parts[0] {
            "blob" => ObjectType::Blob,
            "tree" => ObjectType::Tree,

            // TODO add proper error handling
            _ => ObjectType::Blob,
        };

        let obj_oid = parts[1];
        let obj_name = parts[2];
        entries.push((obj_type, obj_oid.to_string(), obj_name.to_string()));
    }
    entries
}

fn get_tree(oid: &str, base_path: &Path) -> HashMap<PathBuf, String> {
    let mut result = HashMap::new();

    for (type_, entry_oid, name) in iter_tree_entries(oid) {
        assert!(!name.contains('/'));
        assert!(name != "." && name != "..");

        let path = base_path.join(&name);

        match type_.as_str() {
            "blob" => {
                result.insert(path, entry_oid);
            }
            "tree" => {
                let subtree = get_tree(&entry_oid, &path);
                result.extend(subtree);
            }
            _ => panic!("Unknown tree entry type: {}", type_),
        }
    }
    result
}

pub fn read_tree(oid: &str) {
    println!("{:?}", get_tree(oid, Path::new("./")));
    for (entry, oid) in get_tree(oid, Path::new("./")).iter() {
        if entry.is_file() {
            let _ = fs::create_dir_all(entry.parent().unwrap());
            fs::write(entry, get_object(oid, ObjectType::Blob)).unwrap();
        } else {
            let _ = fs::create_dir_all(entry);
        }
    }
}
