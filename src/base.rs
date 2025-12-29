use std::collections::HashSet;
use std::fs::{self};
use std::path::{Path, PathBuf};

use crate::data::{ObjectType, hash_object};

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
    let mut entries: Vec<(String, String, ObjectType)> = Vec::new();

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
                entries.push((file_name, oid, ObjectType::Blob));
            } else if path.is_dir() {
                println!("[dir] {:?}", path);
                let oid = write_tree(&path);
                entries.push((file_name, oid, ObjectType::Tree));
            }
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut tree = String::new();
    entries.iter().for_each(|(name, oid, obj_type)| {
        tree.push_str(format!("{} {} {}\n", name, oid, obj_type.as_str()).as_str());
    });
    hash_object(tree.as_bytes(), ObjectType::Tree)
}
