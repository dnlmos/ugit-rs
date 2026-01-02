use anyhow::{Error, Result};
use clap::{Parser, Subcommand};
use std::fs::{self};

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
    ReadTree {
        #[arg(required = true)]
        tree_oid: String,
    },
    Log,
}

pub fn init_repository(config: &Config) -> Result<(), Error> {
    if fs::exists(config.git_dir.join("objects"))? {
        println!("Repository already initialized");
        Ok(())
    } else {
        println!("Initializing repository {}...", &config.git_dir.display());
        fs::create_dir_all(config.git_dir.join("objects"))?;
        Ok(())
    }
}
