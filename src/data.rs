use std::fs;
use std::hash::{DefaultHasher, Hasher};

use crate::cli::GIT_DIR;

/// Hashes an object from the file at the given path and writes it to the object store.
///
/// # Arguments
///
/// * `file_path` - A string slice that holds the path to the file to be hashed.
///
/// # Returns
///
/// Returns a `Result` indicating success or error.
pub fn hash_object(file_path: &str, type_: ObjectType) -> u64 {
    let content = fs::read_to_string(file_path).expect("failed to read the file");
    let obj = [type_.as_bytes(), b"\x00", content.as_bytes()].concat();

    let mut hasher = DefaultHasher::new();
    hasher.write(&obj);
    let oid = hasher.finish();
    println!("{oid}");

    let path = format!("{}/objects/{}", GIT_DIR, oid);
    fs::write(&path, obj).expect("failed to write object");

    oid
}

pub fn get_object(oid: &u64, expected: ObjectType) -> String {
    let path = format!("{}/objects/{}", GIT_DIR, oid);
    let obj = fs::read(&path).expect("failed to read object");

    let null_pos = obj
        .iter()
        .position(|&b| b == 0)
        .expect("invalid object: no null byte");

    let type_ = str::from_utf8(&obj[..null_pos]).expect("invalid type string");

    assert_eq!(
        type_,
        expected.as_str(),
        "Expected {}, got {}",
        expected.as_str(),
        type_
    );

    str::from_utf8(&obj[null_pos + 1..])
        .expect("error decoding contetnt")
        .to_string()
}

pub enum ObjectType {
    Blob,
}

impl ObjectType {
    /// Get bytes representation of the ObjectType
    fn as_bytes(&self) -> &[u8] {
        match self {
            ObjectType::Blob => "blob".as_bytes(),
        }
    }
    fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_hash_object() -> io::Result<()> {
        // create a temp directory
        let test_dir = format!("{}/objects", GIT_DIR);
        fs::create_dir_all(&test_dir)?;

        // create temp file and write to it
        let file_path = "test_file.txt";
        let content = "Hello world!";
        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;

        let oid = hash_object(file_path, ObjectType::Blob);

        // check if the object file was created
        let object_path = format!("{}/objects/{}", GIT_DIR, oid);
        println!("{}", object_path);
        assert!(Path::new(&object_path).exists());

        // check if object is correct
        let retrieved = get_object(&oid, ObjectType::Blob);
        assert_eq!(retrieved, content);

        // cleanup
        fs::remove_file(file_path)?;
        fs::remove_file(object_path)?;
        fs::remove_dir_all(test_dir)?;

        Ok(())
    }
}
