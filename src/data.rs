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

/// Fetches a reference value.
/// Replicates: get_ref(name, deref=True)
pub fn get_ref(name: &str, follow: &Follow, config: &Config) -> Result<RefTarget> {
    let deref = matches!(follow, Follow::IfSymbolic);
    let (_, ref_value) = get_ref_internal(name, deref, config)?;
    Ok(ref_value)
}

pub fn update_ref(name: &str, target: &RefTarget, follow: &Follow, config: &Config) -> Result<()> {
    let deref = matches!(follow, Follow::IfSymbolic);

    // resolve the path
    // if get_ref_internal fails because the ref is missing,
    // we use the 'name' itself as the target path.
    // we ignore actual content of the file since we overwrite it
    let actual_path = match get_ref_internal(name, deref, config) {
        Ok((path, _)) => path,
        Err(_) => name.to_string(),
    };

    match target {
        RefTarget::Symbolic(path) => println!("Following link to: {}", path),
        RefTarget::Direct(oid) => println!("Found commit hash: {}", oid),
    }

    let full_path = config.git_dir.join(&actual_path);

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    // write the OID (create or overwrite)
    fs::write(&full_path, target.to_string()).with_context(|| {
        format!(
            "Failed to write ref '{}' to {}",
            target,
            full_path.display()
        )
    })?;

    Ok(())
}

/// Resolves a Git reference and returns the OID it points to.
///
/// # Arguments
/// * `ref_` - The reference name (e.g. `refs/heads/@`, `refs/tags/commit123`)
/// * `config` - Repository configuration
///
/// # Returns
/// The object Ref name and RefValue which contains OID and flag if it is symbolic ref.
///
/// # Errors
/// Returns an error if the reference cannot be found or if a reference file exists but cannot be read.
fn get_ref_internal(ref_: &str, deref: bool, config: &Config) -> Result<(String, RefTarget)> {
    let ref_path = config.git_dir.join(ref_);

    let contents = fs::read_to_string(&ref_path)
        .map_err(|e| anyhow!("failed to read ref {}: {}", ref_path.display(), e))?;

    let contents = contents.trim();

    // symbolic ref: "ref: refs/heads/main"
    if let Some(target) = contents.strip_prefix("ref: ") {
        let target = target.trim().to_string();

        println!("&&&& {contents}");

        if deref {
            // Preserve original ref name when dereferencing
            let (_, value) = get_ref_internal(&target, true, config)?;
            return Ok((ref_.to_string(), value));
        }

        return Ok((ref_.to_string(), RefTarget::Symbolic(target)));
    }

    // Direct ref (OID)
    Ok((ref_.to_string(), RefTarget::Direct(contents.to_string())))
}

/// iterate through all refs in refs/tags/
pub fn iter_refs(follow: Follow, config: &Config) -> Result<Vec<(String, RefTarget)>> {
    let ref_path = config.git_dir.join("refs").join("tags");
    let mut entries: Vec<(String, RefTarget)> = Vec::new();

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
    /// Points directly to an Object ID (e.g., a commit hash)
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
    use crate::cli::{Config, init_repository};
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
