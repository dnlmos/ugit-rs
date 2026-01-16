use clap::{Parser, Subcommand};

pub struct Config {
    pub base_dir: std::path::PathBuf,
    pub git_dir: std::path::PathBuf,
}

impl std::default::Default for Config {
    // default paths
    fn default() -> Self {
        let base = std::path::PathBuf::from(".");
        let git = base.join(".ugit");
        Self {
            base_dir: base,
            git_dir: git,
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Name of the command
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new repository
    Init,
    /// Add files to staging
    Add {
        #[arg(required = true)]
        files: Vec<String>,
    },
    /// Commit staged changes
    Commit {
        #[arg(short, long, default_value = "")]
        message: String,
    },
    /// Get diff of two files
    Diff {
        #[arg(required = true)]
        file_a: String,
        #[arg(required = true)]
        file_b: String,
    },
    WriteTree,
    /// arg: oid or tag name
    ReadTree {
        #[arg(required = true)]
        tree_oid: String,
    },
    /// arg: oid or tag name
    Log {
        oid: Option<String>,
    },
    /// arg: oid or tag name
    Checkout {
        #[arg(required = true)]
        oid: String,
    },
    Tag {
        name: String,
        oid: Option<String>,
    },
    K,
    Branch {
        #[arg(required = true)]
        name: String,
        start_point: Option<String>,
    },
}
