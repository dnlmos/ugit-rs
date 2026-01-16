use anyhow::{Context, Error, Result, anyhow};
use sha1::{Digest, Sha1};
use std::{
    fmt::{self},
    fs,
};

use crate::cli::Config;

/// Hashes object content and stores it in the Git object database.
///
/// Constructs a header with the object type, concatenates it with the content,
/// computes the SHA-1 hash, and writes the full object to `.ugit/objects`.
///
/// # Arguments
/// * `content` - The raw object data.
/// * `type_` - The type of the Git object (blob, tree).
/// * `config` - Configuration containing the `.ugit` directory path.
///
/// # Returns
/// The SHA-1 object ID (OID) as a hexadecimal string on success.
///
/// # Errors
/// Returns an error if writing the object to disk fails.
pub fn hash_object(content: &[u8], type_: ObjectType, config: &Config) -> Result<String, Error> {
    let header = format!("{}\0", type_.as_str());
    let obj = [header.as_bytes(), content].concat();

    let mut hasher = Sha1::new();
    hasher.update(&obj);
    let result = hasher.finalize();
    let oid = hex::encode(result);

    let path = config.git_dir.join("objects").join(&oid);
    fs::write(path, obj)?;

    Ok(oid)
}

/// Reads and validates a Git object from the database.
///
/// Loads an object by its SHA-1 ID, verifies its type, and returns the raw content.
///
/// # Arguments
/// * `oid` - The object ID (SHA-1 hash) to retrieve.
/// * `expected` - The expected type of the object (blob, tree).
/// * `config` - Configuration containing the `.ugit` directory path.
///
/// # Returns
/// The object's content (after the header) on success.
///
/// # Errors
/// Returns an error if:
/// - The object file cannot be read.
/// - The object has no null byte (invalid format).
/// - The object type does not match `expected`.
pub fn get_object(oid: &str, expected: ObjectType, config: &Config) -> Result<Vec<u8>> {
    let path = config.git_dir.join("objects").join(oid);

    let obj = fs::read(&path)
        .with_context(|| format!("Failed to read object '{}' at {:?}", oid, path))?;

    let null_pos = obj
        .iter()
        .position(|&b| b == 0)
        .with_context(|| format!("Object '{}': missing null terminator in header", oid))?;

    let type_str = str::from_utf8(&obj[..null_pos])
        .with_context(|| format!("Object '{}' has an invalid UTF-8 header", oid))?;

    assert_eq!(
        type_str,
        expected.as_str(),
        "Expected {}, got {}",
        expected.as_str(),
        type_str
    );

    Ok(obj[null_pos + 1..].to_owned())
}

/// Fetches a reference value (the OID).
pub fn get_ref(name: &str, follow: &Follow, config: &Config) -> Result<String> {
    let deref = matches!(follow, Follow::IfSymbolic);

    let (_, ref_target) = get_ref_internal(name, deref, config)
        .with_context(|| format!("Failed to resolve reference '{}'", name))?;

    match ref_target {
        Some(RefTarget::Direct(oid)) => Ok(oid),
        Some(RefTarget::Symbolic(path)) => Ok(path),
        None => Err(anyhow!(
            "Reference '{}' exists, but doesnt have OID yet",
            name
        )),
    }
}

/// Resolves a reference name to its target.
///
/// If `deref` is true, it recursively follows "ref: " pointers until it hits an OID.
/// Returns the final path visited and the `RefTarget` (Direct, Symbolic, or None if missing).
fn get_ref_internal(
    ref_name: &str,
    deref: bool,
    config: &Config,
) -> Result<(String, Option<RefTarget>)> {
    let ref_path = config.git_dir.join(ref_name);

    if !ref_path.exists() {
        if deref {
            // If dereferencing, we return the name so it can be created/updated later
            return Ok((ref_name.to_string(), None));
        }
        return Err(anyhow!("Reference '{}' does not exist", ref_name));
    }

    let contents = fs::read_to_string(&ref_path)
        .with_context(|| format!("Failed to read reference file at {:?}", ref_path))?
        .trim()
        .to_string();

    // handle symbolic refs
    if let Some(target_path) = contents.strip_prefix("ref: ") {
        let target_path = target_path.trim();

        if deref {
            // recursion to find the actual OID
            return get_ref_internal(target_path, true, config);
        }

        return Ok((
            ref_name.to_string(),
            Some(RefTarget::Symbolic(target_path.to_string())),
        ));
    }
    Ok((ref_name.to_string(), Some(RefTarget::Direct(contents))))
}

pub fn update_ref(
    name: &str,
    ref_value: &RefTarget,
    follow: &Follow,
    config: &Config,
) -> Result<()> {
    let deref = matches!(follow, Follow::IfSymbolic);
    // find the actual file we need to write to.
    // get_ref_internal(..., deref: true) will follow symbolic links
    // until it finds a direct ref or a path that doesn't exist yet.
    let (target_path, _) = get_ref_internal(name, deref, config)
        .with_context(|| format!("Failed to resolve reference path for '{}'", name))?;

    let full_path = config.git_dir.join(&target_path);

    // ensure the directory exists
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory structure for {:?}", parent))?;
    }

    println!("Writing '{}' to '{}'", ref_value, full_path.display());
    // write oid or ref to file
    fs::write(&full_path, format!("{}", ref_value))
        .with_context(|| format!("Failed to write ref value to reference at {:?}", full_path))?;

    Ok(())
}

/// iterate through all refs in refs/tags/
pub fn iter_refs(follow: Follow, config: &Config) -> Result<Vec<(String, String)>> {
    let ref_path = config.git_dir.join("refs").join("tags");
    let mut entries: Vec<(String, String)> = Vec::new();

    for entry in fs::read_dir(&ref_path)? {
        let entry = entry?;
        if entry.path().is_file() {
            let filename = entry.file_name().to_string_lossy().into_owned();
            if let Some(path_str) = ref_path.join(&filename).to_str() {
                entries.push((filename, get_ref(path_str, &follow, config)?));
            }
        }
    }

    Ok(entries)
}

pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

impl ObjectType {
    pub fn _as_bytes(&self) -> &[u8] {
        match self {
            ObjectType::Blob => "blob".as_bytes(),
            ObjectType::Tree => "tree".as_bytes(),
            ObjectType::Commit => "commit".as_bytes(),
        }
    }
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
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

pub enum RefTarget {
    /// Points directly to an OID
    Direct(String),
    /// Points to another reference (e.g., "ref: refs/heads/master")
    Symbolic(String),
}

impl fmt::Display for RefTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefTarget::Direct(value) => write!(f, "{value}"),
            RefTarget::Symbolic(value) => write!(f, "ref: {value}"),
        }
    }
}

pub enum Follow {
    /// Follow symbolic refs to the ultimate target (deref=True)
    IfSymbolic,
    /// Act on the ref itself (deref=False)
    Never,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::init_repository;
    use crate::cli::Config;
    use tempfile::tempdir;

    #[test]
    fn test_hash_object() -> Result<()> {
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        let config = Config {
            base_dir: temp_dir.path().to_path_buf(),
            git_dir: temp_dir.path().join(".ugit"),
        };
        init_repository(&config)?;

        let content = "Hello world!";

        let oid = hash_object(content.as_bytes(), ObjectType::Blob, &config)
            .expect("Failed to hash object");

        let path = config.git_dir.join("objects").join(&oid);
        assert!(path.exists());

        let retrieved =
            get_object(oid.as_str(), ObjectType::Blob, &config).expect("Failed to retrieve object");

        assert_eq!(retrieved, content.as_bytes());

        Ok(())
    }
}
