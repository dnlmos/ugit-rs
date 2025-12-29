use std::collections::{HashMap, HashSet};
use std::fs::{self};
use std::path::{Path, PathBuf};

use ugit_rs::cli::{BASE_DIR, GIT_DIR};

use crate::data::{ObjectType, get_object, hash_object};

fn get_ignored_files() -> HashSet<PathBuf> {
    let mut ignored_files = HashSet::new();

    if let Ok(content) = fs::read_to_string(BASE_DIR.to_string() + "/.ugitignore") {
        content
            .lines()
            .filter(|line| !line.is_empty())
            .for_each(|entry| {
                ignored_files.insert(Path::new(BASE_DIR).join(Path::new(entry)));
            });
    }
    println!("[ignored files] {:?}", ignored_files);
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

            // TODO error handling
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
    } else {
        // TODO error handling
        println!("err reading {}", dir.to_str().unwrap());
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

fn get_tree(oid: &str, base_path: &Path) -> HashMap<PathBuf, (ObjectType, String)> {
    let mut result = HashMap::new();

    for (obj_type, entry_oid, name) in iter_tree_entries(oid) {
        let path = base_path.join(&name);

        let is_tree = matches!(obj_type, ObjectType::Tree);

        result.insert(path.clone(), (obj_type, entry_oid.clone()));

        if is_tree {
            let subtree = get_tree(&entry_oid, &path);
            result.extend(subtree);
        }
    }
    result
}

pub fn read_tree(oid: &str) {
    let tree_map = get_tree(oid, Path::new(BASE_DIR));

    for (path, (obj_type, entry_oid)) in tree_map {
        match obj_type {
            ObjectType::Blob => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).ok();
                }

                println!(
                    "[creating file] {} | oid {}",
                    path.to_str().unwrap(),
                    entry_oid
                );
                let data = get_object(&entry_oid, ObjectType::Blob);

                fs::write(path, data).expect("Failed to write blob");
            }
            ObjectType::Tree => {
                println!("[creating dir] {}", path.to_str().unwrap());
                fs::create_dir_all(path).expect("Failed to create directory");
            }
        }
    }
}
