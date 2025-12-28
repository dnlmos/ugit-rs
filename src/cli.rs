use clap::{Parser, Subcommand};
// use ugit_rs::data::hash_object;
use std::fs::{self};
use std::io::{self};

pub const GIT_DIR: &str = ".ugit";

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
}

pub fn init_repository(dir: &str) -> io::Result<()> {
    if fs::exists(dir).is_ok() {
        println!("Repository already initialized");
        Ok(())
    } else {
        println!("Initializing repository {}...", dir);
        fs::create_dir_all(format!("{}/objects", dir))
    }
}
