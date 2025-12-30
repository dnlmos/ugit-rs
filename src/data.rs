use sha1::{Digest, Sha1};
use std::{fmt, fs};

use crate::cli::GIT_DIR;

/// Hashes an object from the given content and writes it to the object store.
///
/// # Arguments
///
/// * `content` - The raw byte content of the object.
/// * `type_` - The type of Git object (e.g., Blob, Tree).
///
/// # Returns
///
/// Returns the hexadecimal SHA-1 hash (OID) of the stored object.
pub fn hash_object(content: &[u8], type_: ObjectType) -> String {
    let header = format!("{}\0", type_.as_str());
    let obj = [header.as_bytes(), content].concat();

    let mut hasher = Sha1::new();
    hasher.update(&obj);
    let result = hasher.finalize();

    let oid = hex::encode(result);

    let path = format!("{}/objects/{}", GIT_DIR, oid);
    fs::write(path, obj).unwrap();

    oid
}

/// Reads and validates a Git object from the object store.
///
/// # Arguments
///
/// * `oid` - The hexadecimal object ID (OID) to retrieve.
/// * `expected` - The expected type of the object (e.g., Blob, Tree).
///
/// # Returns
///
/// Returns the decoded content of the object (excluding header).
pub fn get_object(oid: &str, expected: ObjectType) -> Vec<u8> {
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

    obj[null_pos + 1..].to_owned()
}

pub enum ObjectType {
    Blob,
    Tree,
}

impl ObjectType {
    /// Get bytes representation of the ObjectType
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            ObjectType::Blob => "blob".as_bytes(),
            ObjectType::Tree => "tree".as_bytes(),
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::init_repository;

    use super::*;
    use std::io;
    use std::path::Path;

    #[test]
    fn test_hash_object() -> io::Result<()> {
        if !Path::new(format!("{}/objects", GIT_DIR).as_str()).exists() {
            init_repository()?;
        }
        let content = "Hello world!";

        let oid = hash_object(content.as_bytes(), ObjectType::Blob);

        // check if the object file was created
        let object_path = format!("{}/objects/{}", GIT_DIR, oid);
        assert!(Path::new(&object_path).exists());

        // check if object is correct
        let retrieved = get_object(oid.as_str(), ObjectType::Blob);
        assert_eq!(retrieved, content.as_bytes());

        // cleanup
        fs::remove_file(object_path)?;

        Ok(())
    }
}
