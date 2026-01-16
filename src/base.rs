use crate::data::{
    Follow, ObjectType, RefTarget, get_object, get_ref, hash_object, iter_refs, update_ref,
};
use crate::utils::is_valid_sha1;
use anyhow::anyhow;
use colored::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::{self, Write as _};
use std::fs::{self};
use std::path::{Path, PathBuf};

use anyhow::{Context, Error, Result};
use walkdir::WalkDir;

use crate::cli::Config;

pub struct Commit {
    pub tree: String,
    pub parent: Option<String>,
    pub message: String,
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "tree {}", self.tree)?;
        if let Some(ref p) = self.parent {
            writeln!(f, "parent {}", p)?;
        }
        writeln!(f)?;
        write!(f, "{}", self.message)
    }
}

pub fn init_repository(config: &Config) -> Result<(), Error> {
    if fs::exists(config.git_dir.join("objects"))? {
        println!("Repository already initialized");
    } else {
        println!("Initializing repository {}...", &config.git_dir.display());
        fs::create_dir_all(config.git_dir.join("objects"))?;
    };

    // create master branch
    update_ref(
        "HEAD",
        &RefTarget::Symbolic("refs/heads/master".to_string()),
        &Follow::IfSymbolic,
        config,
    )
}

/// Reads and returns a set of ignored file paths from `.ugitignore`.
///
/// Loads patterns from `.ugitignore`, trims and normalizes them, and resolves each
/// to an absolute path relative to the base directory.
///
/// # Arguments
/// * `config` - Configuration containing the project's base directory.
///
/// # Returns
/// A set of ignored file paths, or an empty set if no ignore file exists.
fn get_ignored_files(config: &Config) -> HashSet<PathBuf> {
    let mut ignored_files = HashSet::new();

    if let Ok(content) = fs::read_to_string(config.base_dir.join(".ugitignore")) {
        content
            .lines()
            .filter(|line| !line.is_empty())
            .for_each(|entry| {
                ignored_files.insert(config.base_dir.join(Path::new(entry.trim())));
            });
    }
    ignored_files
}

/// Recursively writes a directory tree to the Git object database.
///
/// Reads all non-ignored entries in a directory, hashes files and subdirectories,
/// and creates a tree object with sorted entries.
///
/// # Arguments
/// * `dir` - The directory to serialize into a tree.
/// * `config` - Configuration containing the `.ugit` directory path.
///
/// # Returns
/// The SHA-1 ID of the resulting tree object.
///
/// # Errors
/// Returns an error if reading the directory, files, or subdirectories fails,
/// or if hashing objects fails.
pub fn write_tree(dir: &Path, config: &Config) -> Result<String> {
    let ignored_files = get_ignored_files(config);
    let mut entries: Vec<(ObjectType, String, String)> = Vec::new();

    let read_dir =
        fs::read_dir(dir).context(format!("Failed to read directory: {}", dir.display()))?;

    for entry in read_dir {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // skip ignored files/dirs
        if ignored_files.contains(&path) {
            continue;
        }

        let file_name = path
            .file_name()
            .context(format!("Invalid filename for path: {}", path.display()))?
            .to_str()
            .context(format!("Not utf8 filename: {}", path.display()))?
            .to_string();

        if path.is_file() {
            let content =
                fs::read(&path).context(format!("Failed to read file: {}", path.display()))?;
            let oid = hash_object(&content, ObjectType::Blob, config)?;
            entries.push((ObjectType::Blob, oid, file_name));
        } else if path.is_dir() {
            let oid = write_tree(&path, config)?;
            entries.push((ObjectType::Tree, oid, file_name));
        }
    }

    entries.sort_by(|a, b| a.2.cmp(&b.2));

    let mut tree = String::new();
    for (obj_type, oid, name) in &entries {
        tree.push_str(&format!("{} {} {}\n", obj_type.as_str(), oid, name));
    }

    hash_object(tree.as_bytes(), ObjectType::Tree, config)
}

/// Parses a tree object and returns its entries.
///
/// Reads a tree object by ID, decodes its content, and extracts type, OID, and name for each entry.
///
/// # Arguments
/// * `oid` - The SHA-1 ID of the tree object.
/// * `config` - Configuration containing the `.ugit` directory path.
///
/// # Returns
/// A list of tuples containing (object type, OID, filename) for each entry in the tree.
///
/// # Errors
/// Returns an error if reading the object fails, is unsupported or if the content is malformed.
fn iter_tree_entries(oid: &str, config: &Config) -> Result<Vec<(ObjectType, String, String)>> {
    let tree = get_object(oid, ObjectType::Tree, config)?;
    let content = str::from_utf8(&tree)
        .with_context(|| format!("Failed to retrieve commit object '{}' from storage", oid))?;
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
            _ => return Err(anyhow!("Encountered unsupported object type: {}", parts[0])),
        };

        let obj_oid = parts[1];
        let obj_name = parts[2];
        entries.push((obj_type, obj_oid.to_string(), obj_name.to_string()));
    }
    Ok(entries)
}

/// Recursively retrieves all entries in a tree and its sub-trees.
///
/// Walks a tree object and its nested trees, building a map of paths to their object types and OIDs.
///
/// # Arguments
/// * `oid` - The SHA-1 ID of the root tree object.
/// * `base_path` - The base path to resolve relative file paths.
/// * `config` - Configuration containing the `.ugit` directory path.
///
/// # Returns
/// A map from file/directory paths to their object type and OID.
///
/// # Errors
/// Returns an error if reading tree entries or nested trees fails.
fn get_tree(
    oid: &str,
    base_path: &Path,
    config: &Config,
) -> Result<HashMap<PathBuf, (ObjectType, String)>, Error> {
    let mut result = HashMap::new();

    for (obj_type, entry_oid, name) in iter_tree_entries(oid, config)? {
        let path = base_path.join(&name);

        let is_tree = matches!(obj_type, ObjectType::Tree);

        result.insert(path.clone(), (obj_type, entry_oid.clone()));

        if is_tree {
            let subtree = get_tree(&entry_oid, &path, config)?;
            result.extend(subtree);
        }
    }
    Ok(result)
}

/// Clears the working directory by removing all tracked files and directories.
///
/// Deletes all files and directories under the base directory, excluding those in `.ugitignore`.
/// Processes entries in bottom-up order to safely remove nested structures.
///
/// # Arguments
/// * `config` - Configuration containing the project's base directory and ignore list.
///
/// # Behavior
/// Ignores errors during deletion (e.g., permission issues or missing files).
fn empty_working_dir(config: &Config) {
    let ignored_files = get_ignored_files(config);
    for entry in WalkDir::new(&config.base_dir)
        .contents_first(true) // bottom-up (first files, then corresponding dir)
        .min_depth(1) // skip the BASE_DIR
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            // filter out ignored files and paths that contains ignored dirs
            // TODO implement better globbing
            let path = e.path();
            !ignored_files
                .iter()
                .any(|ignored| path.starts_with(ignored))
        })
    {
        let path = entry.path();

        // since it may contain ignored file, ignore errors
        if path.is_file() {
            let _ = fs::remove_file(path);
        } else if path.is_dir() {
            let _ = fs::remove_dir(path);
        }
    }
}

/// Restores the working directory from a tree object.
///
/// Clears the current working directory, then recreates all files and directories
/// from the given tree OID according to the object database.
///
/// # Arguments
/// * `oid` - The SHA-1 ID of the tree object to restore.
/// * `config` - Configuration containing base directory and `.ugit` path.
///
/// # Returns
/// `Ok(())` on success, or an error if reading objects or writing files fails.
pub fn read_tree(oid: &str, config: &Config) -> Result<()> {
    empty_working_dir(config);
    let tree_map = get_tree(oid, &config.base_dir, config)?;

    for (path, (obj_type, entry_oid)) in tree_map {
        match obj_type {
            ObjectType::Blob => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                let data = get_object(&entry_oid, ObjectType::Blob, config)?;
                fs::write(&path, data)
                    .with_context(|| format!("Failed to restore file: {:?}", path.display()))?;
            }
            ObjectType::Tree => {
                fs::create_dir_all(&path)
                    .with_context(|| format!("Failed to restore file: {:?}", path.display()))?;
            }
            _ => return Err(anyhow!("Encountered unsupported object type: {}", obj_type)),
        }
    }
    Ok(())
}

/// # Example
/// ```
/// tree 5e550586c91fce59e0006799e0d46b3948f05693
/// parent .........
///
/// This is the commit message!
/// ```
/// # Returns
/// oid of the commit object
pub fn create_commit(message: String, config: &Config) -> Result<String> {
    let tree_hash = write_tree(&config.base_dir, config)?;
    let mut commit = format!("tree {}\n", tree_hash);

    if let Ok(oid) = get_ref("HEAD", &Follow::IfSymbolic, config) {
        writeln!(&mut commit, "parent {}", oid)?;
    }

    commit.push_str(&format!("\n{}\n", message));
    let commit_oid = hash_object(commit.as_bytes(), ObjectType::Commit, config)?;

    update_ref(
        "HEAD",
        &RefTarget::Direct(commit_oid.clone()),
        &Follow::IfSymbolic,
        config,
    )?;

    Ok(commit_oid)
}

pub fn get_commit(oid: &str, config: &Config) -> Result<Commit> {
    let bytes = get_object(oid, ObjectType::Commit, config)
        .with_context(|| format!("Failed to retrieve commit object '{}' from storage", oid))?;

    let commit_str = std::str::from_utf8(&bytes)
        .with_context(|| format!("Commit '{}' contains invalid UTF-8 data", oid))?;
    let mut lines = commit_str.lines().enumerate();

    let mut tree = "";
    let mut parent = None;
    for (idx, line) in lines.by_ref() {
        // commit message is after empty line
        if line.is_empty() {
            break;
        }

        let mut words = line.split_whitespace();
        let key = words.next();
        let value = words.next();
        let extra = words.next();

        match (key, value, extra) {
            (Some(_k), Some(_v), None) => match _k {
                "tree" => tree = _v,
                "parent" => parent = Some(_v.to_string()),
                _ => {
                    return Err(anyhow!(
                        "Invalid commit header at line {}: expected 'tree or parent', found '{}'",
                        idx + 1,
                        line
                    ));
                }
            },
            _ => {
                return Err(anyhow!(
                    "Invalid commit header at line {}: expected 'key value', found '{}'",
                    idx + 1,
                    line
                ));
            }
        }
    }

    let message_lines: Vec<String> = lines.map(|(_, line)| line.to_string()).collect();
    let message = message_lines.join("\n");

    Ok(Commit {
        tree: tree.to_string(),
        parent,
        message: message.to_string(),
    })
}

/// Returns the commit history starting from the given revision.
///
/// `oid` may be an object ID or a resolvable name (tag).
pub fn log(oid: &str, config: &Config) -> Result<String> {
    let mut history = String::new();
    let mut current_oid: Option<String> = Some(resolve_oid(oid, config)?);

    while let Some(oid) = current_oid {
        let commit = get_commit(&oid, config)
            .with_context(|| format!("Failed to read history at {}", oid))?;
        writeln!(history, "{} {}", "commit".yellow(), oid.yellow().bold())?;
        for line in commit.message.lines() {
            writeln!(history, "     | {}", line)?;
        }
        writeln!(history, "     |")?;
        writeln!(history)?;
        current_oid = commit.parent;
    }
    Ok(history)
}

pub fn resolve_oid(name: &str, config: &Config) -> Result<String> {
    if is_valid_sha1(name) {
        return Ok(name.to_string());
    }
    get_ref(name, &Follow::IfSymbolic, config)
}

pub fn checkout(name: &str, config: &Config) -> Result<()> {
    let oid = get_oid(name, config);
    let commit = get_commit(&oid, config)
        .with_context(|| format!("Error reading commit '{}' with oid '{}'", name, oid))?;
    read_tree(&commit.tree, config)
        .with_context(|| format!("Error reading tree '{}'", commit.tree))?;

    let head = match is_branch(name, config) {
        true => RefTarget::Symbolic(format!("refs/heads/{name}")),
        false => RefTarget::Direct(oid),
    };

    match head {
        // Update HEAD to point to branch name
        RefTarget::Symbolic(_) => update_ref("HEAD", &head, &Follow::Never, config),
        // Update HEAD to point directly to the commit
        RefTarget::Direct(_) => update_ref("HEAD", &head, &Follow::IfSymbolic, config),
    }
}

fn is_branch(name: &str, config: &Config) -> bool {
    let path = format!("refs/heads/{name}");
    get_ref(&path, &Follow::Never, config).is_ok()
}

pub fn create_tag(name: &str, oid: &str, config: &Config) -> Result<()> {
    update_ref(
        &format!("refs/tags/{}", name),
        &RefTarget::Direct(oid.to_string()),
        &Follow::IfSymbolic,
        config,
    )
}

// Return formatted output of git history
pub fn k(config: &Config) -> Result<String> {
    let mut output = String::new();
    let mut refs_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut oids = BTreeSet::new();

    for entry in iter_refs(Follow::IfSymbolic, config).iter() {
        for x in entry.iter() {
            refs_map
                .entry(x.1.to_string())
                .or_default()
                .push(x.0.clone());
            oids.insert(x.1.to_string());
        }
    }

    let commits = iter_comits_and_parents(oids, config)?;

    output.push('\n');

    for (i, oid) in commits.iter().enumerate() {
        let commit = get_commit(oid, config)?;
        let short_oid = &oid[..10.min(oid.len())];

        let is_last = i == commits.len() - 1;
        let prefix = if is_last { "└─" } else { "├─" };
        let continuation = if is_last { "  " } else { "│ " };

        // Show the commit
        output.push_str(&format!("{} ● {}", prefix, short_oid.yellow()));

        // Show refs
        if let Some(refs) = refs_map.get(oid) {
            output.push_str(&format!(
                " ← {} | {}",
                refs.join(", "),
                commit.message.red()
            ));
        }

        output.push('\n');

        // Add spacing for next commit
        if !is_last {
            output.push_str(&format!("{}   \n", continuation));
        }
    }

    output.push('\n');
    Ok(output)
}

/// Resolve a "name" to an OID. A name can either be a ref (in which case this
/// function will return the OID that the ref points to) or an OID
/// (in which case get_oid will just return that same OID).
pub fn get_oid(name: &str, config: &Config) -> String {
    let refs_to_try = [
        name,
        &format!("refs/{}", name),
        &format!("refs/tags/{}", name),
        &format!("refs/heads/{}", name),
    ];

    for path in refs_to_try {
        if let Ok(oid) = get_ref(path, &Follow::IfSymbolic, config) {
            return oid;
        }
    }

    name.to_string()
}

pub fn iter_comits_and_parents(
    oids: BTreeSet<String>,
    config: &Config,
) -> Result<BTreeSet<String>> {
    let mut oids: VecDeque<String> = oids.into_iter().collect();
    let mut visited: BTreeSet<String> = BTreeSet::new();

    while !oids.is_empty() {
        let oid_ = oids.pop_front();
        if let Some(oid) = oid_ {
            // oids.push(oid.clone()); // ???
            if !visited.contains(&oid) {
                visited.insert(oid.to_owned());
                let commit = get_commit(&oid, config)?.parent;
                if let Some(parent) = commit {
                    oids.push_front(parent);
                }
            }
        }
    }

    Ok(visited)
}

pub fn create_branch(name: &str, start_oid: &str, config: &Config) -> Result<()> {
    let ref_path = format!("refs/heads/{name}");
    // branches and tags are direct references
    update_ref(
        &ref_path,
        &RefTarget::Direct(start_oid.to_string()),
        &Follow::IfSymbolic,
        config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::init_repository;
    use crate::cli::Config;
    use anyhow::Ok;
    use tempfile::{TempDir, tempdir};

    fn create_test_repo() -> Result<(TempDir, Config)> {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let config = Config {
            base_dir: temp_dir.path().to_path_buf(),
            git_dir: temp_dir.path().join(".ugit"),
        };
        init_repository(&config)?;
        Ok((temp_dir, config))
    }

    /// create some file structure, generated by AI
    fn create_test_file_structure(base_dir: &Path) -> Result<()> {
        fs::write(base_dir.join("file1.txt"), "Hello, World!")?;
        fs::write(base_dir.join("file2.txt"), "Test content")?;
        fs::write(
            base_dir.join("README.md"),
            "# My Project\n\nThis is a test.",
        )?;

        // create a directory with files
        let sub_dir = base_dir.join("src");
        fs::create_dir(&sub_dir)?;
        fs::write(
            sub_dir.join("main.rs"),
            "fn main() {\n    println!(\"Hello\");\n}",
        )?;
        fs::write(
            sub_dir.join("lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}",
        )?;

        // Create a nested directory
        let nested_dir = sub_dir.join("utils");
        fs::create_dir(&nested_dir)?;
        fs::write(nested_dir.join("helper.rs"), "pub fn helper() {}")?;
        fs::write(
            nested_dir.join("helper_ignored.rs"),
            "pub fn helper_ignored() {}",
        )?;

        fs::write(
            base_dir.join(".ugitignore"),
            ".ugit/\n .ugitignore\n src/utils/helper_ignored.rs\n",
        )?;

        Ok(())
    }

    #[test]
    fn test_writing_tree() -> Result<()> {
        let (temp_dir, config) = create_test_repo().expect("Failed to create test repository");
        create_test_file_structure(temp_dir.path()).expect("Failed to create file structure");

        let tree_oid = write_tree(temp_dir.path(), &config)?;

        assert!(!tree_oid.is_empty());

        // Verify the tree object was created
        let tree_path = config.git_dir.join("objects").join(&tree_oid);
        assert!(tree_path.exists());

        Ok(())
    }

    #[test]
    fn test_reading_tree() -> Result<()> {
        let (temp_dir, config) = create_test_repo().expect("Failed to create test repository");
        create_test_file_structure(temp_dir.path()).expect("Failed to create file structure");

        let mut entries_before = get_repository_contents(&config)?;
        entries_before.sort();
        let tree_oid = write_tree(temp_dir.path(), &config)?;
        read_tree(&tree_oid, &config)?;
        let mut entries_after = get_repository_contents(&config)?;
        entries_after.sort();

        assert_eq!(entries_before, entries_after);

        Ok(())
    }

    /// With respect to .ugitignore
    fn get_repository_contents(config: &Config) -> Result<Vec<PathBuf>> {
        let ignored_files = get_ignored_files(config);
        let mut entries: Vec<PathBuf> = Vec::new();
        for entry in WalkDir::new(&config.base_dir)
            .contents_first(true) // bottom-up (first files, then corresponding dir)
            .min_depth(1) // skip the BASE_DIR
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                // filter out ignored files and paths that contains ignored dirs
                // TODO implement better globbing
                let path = e.path();
                !ignored_files
                    .iter()
                    .any(|ignored| path.starts_with(ignored))
            })
        {
            entries.push(entry.into_path());
        }
        Ok(entries)
    }

    #[test]
    fn test_create_and_checkout_integrity() -> Result<()> {
        let (temp_dir, config) = create_test_repo().expect("Failed to create test repository");

        // first commit
        create_test_file_structure(temp_dir.path())?;
        let first_oid = create_commit("First message".to_string(), &config)?;
        let mut state_one = get_repository_contents(&config)?;
        state_one.sort();

        // add extra file and create second commit
        std::fs::write(temp_dir.path().join("extra.txt"), "new content")?;
        let second_oid = create_commit("Second message".to_string(), &config)?;
        let mut state_two = get_repository_contents(&config)?;
        state_two.sort();

        // checkout first commit and compare file system
        checkout(&first_oid, &config)?;
        let mut current_entries = get_repository_contents(&config)?;
        current_entries.sort();

        assert_eq!(
            get_ref("HEAD", &Follow::IfSymbolic, &config)?.to_string(),
            first_oid
        );
        assert_eq!(
            current_entries, state_one,
            "FS should match first commit state"
        );

        // checkout second commit
        checkout(&second_oid, &config)?;
        let mut current_entries = get_repository_contents(&config)?;
        current_entries.sort();

        assert_eq!(
            get_ref("HEAD", &Follow::IfSymbolic, &config)?.to_string(),
            second_oid
        );
        assert_eq!(
            current_entries, state_two,
            "FS should match Second Commit state"
        );

        Ok(())
    }

    #[test]
    fn test_tags() -> Result<()> {
        let (temp_dir, config) = create_test_repo().expect("failed to create test repository");

        // first commit
        create_test_file_structure(temp_dir.path())?;
        let first_oid = create_commit("first message".to_string(), &config)
            .context("error creating first commit")?;
        create_tag("first commit", &first_oid, &config)?;
        let mut state_one = get_repository_contents(&config)?;
        state_one.sort();

        // add extra file and create second commit
        std::fs::write(temp_dir.path().join("extra.txt"), "new content")?;
        let second_oid = create_commit("second message".to_string(), &config)?;
        create_tag("second commit", &second_oid, &config)?;

        // check if tag is created and contains correct oid
        assert_eq!(
            fs::read_to_string(config.git_dir.join("refs/tags/second commit"))?,
            second_oid
        );
        // check if head has the correct oid as "second commit"
        assert_eq!(
            fs::read_to_string(config.git_dir.join("refs/heads/master"))?,
            second_oid
        );

        let mut state_two = get_repository_contents(&config)?;
        state_two.sort();

        // checkout first commit and compare file system
        checkout("first commit", &config)?;
        let mut current_entries = get_repository_contents(&config)?;
        current_entries.sort();

        // check if head has the correct oid as "first commit"
        assert_eq!(
            fs::read_to_string(config.git_dir.join("refs/tags/first commit"))?,
            first_oid
        );

        // // check if tag is created and contains correct oid
        // assert_eq!(
        //     fs::read_to_string(config.git_dir.join("refs/heads/master"))?,
        //     first_oid
        // );

        assert_eq!(
            current_entries, state_one,
            "FS should match first commit state"
        );
        Ok(())
    }

    #[test]
    fn test_k() -> Result<()> {
        let (temp_dir, config) = create_test_repo().expect("failed to create test repository");

        // first commit
        create_test_file_structure(temp_dir.path())?;
        let first_oid = create_commit("first message".to_string(), &config)?;
        create_tag("first commit", &first_oid, &config)?;
        // add extra file and create second commit
        std::fs::write(temp_dir.path().join("extra.txt"), "new content")?;
        let second_oid = create_commit("second message".to_string(), &config)?;
        create_tag("second commit", &second_oid, &config)?;
        // add extra file and create third commit
        std::fs::write(
            temp_dir.path().join("extra_third.txt"),
            "new content for third commit",
        )?;
        let _third_oid = create_commit("third message".to_string(), &config)?;
        create_tag("third commit", &second_oid, &config)?;

        println!("{}", k(&config)?);

        Ok(())
    }
    #[test]
    fn test_branches() -> Result<()> {
        let (temp_dir, config) = create_test_repo().expect("failed to create test repository");
        create_test_file_structure(temp_dir.path())?;

        // initial commit on master branch
        let first_oid = create_commit("Initial commit".to_string(), &config)?;
        create_branch("feature", &first_oid, &config)?;

        checkout("feature", &config)?;

        // second commit on feature branch
        std::fs::write(temp_dir.path().join("feature_logic.txt"), "feature data")?;
        let second_oid = create_commit("Feature commit".to_string(), &config)?;

        let master_oid = get_ref("refs/heads/master", &Follow::Never, &config)?;
        let feature_oid = get_ref("refs/heads/feature", &Follow::Never, &config)?;
        let head_resolved = get_ref("HEAD", &Follow::IfSymbolic, &config)?;

        assert_eq!(master_oid, first_oid, "Master should not have moved");
        assert_eq!(
            feature_oid, second_oid,
            "Feature branch should have updated"
        );
        assert_eq!(
            head_resolved, second_oid,
            "HEAD should resolve to second commit"
        );

        Ok(())
    }
}
