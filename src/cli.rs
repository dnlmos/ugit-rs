use clap::{Parser, Subcommand};
use std::fs::{self};
use std::io::{self};

pub const BASE_DIR: &str = "./test_dir";
pub const GIT_DIR: &str = "./test_dir/.ugit";

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
    ReadTree,
}

pub fn init_repository() -> io::Result<()> {
    if fs::exists(GIT_DIR)? {
        println!("Repository already initialized");
        Ok(())
    } else {
        println!("Initializing repository {}...", GIT_DIR);
        fs::create_dir_all(format!("{}/objects", GIT_DIR))
    }
}
