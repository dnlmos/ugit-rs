use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;

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
pub fn hash_object(file_path: &str) -> io::Result<()> {
    let content = fs::read_to_string(file_path)?;

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let oid = hasher.finish();

    fs::write(format!("{GIT_DIR}/objects/{oid}"), content)
}

pub fn get_object(oid: &str) -> io::Result<String> {
    fs::read_to_string(format!("{}/objects/{}", GIT_DIR, oid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn test_hash_object() -> io::Result<()> {
        // Setup: Create a temporary directory
        let test_dir = format!("{}/objects", GIT_DIR);
        fs::create_dir_all(&test_dir)?;

        // Create a temporary file
        let file_path = "test_file.txt";
        let content = "Hello, Git!";

        // Write content to the temporary file
        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;

        // Call the hash_object function
        hash_object(file_path)?;

        // Check if the object file was created
        let oid = {
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            hasher.finish()
        };
        let object_path = format!("{}/objects/{}", GIT_DIR, oid);

        assert!(Path::new(&object_path).exists());

        // cleanup
        fs::remove_file(file_path)?;
        fs::remove_dir_all(test_dir)?;

        Ok(())
    }
}
