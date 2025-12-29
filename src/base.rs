use std::collections::HashSet;
use std::fs::{self};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

// Examples are from [https://rustwiki.org/en/rust-cookbook/file/dir.html]
pub fn write_tree(dir: &str) {
    let ignored_files = get_ignored_files();
    WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !ignored_files.contains(e.path()))
        .filter_map(|v| v.ok()) // write file to object store
        .for_each(|x| println!("{}", x.path().display()));

    println!("{:?}", ignored_files);
}
