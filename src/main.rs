use clap::{Parser, Subcommand};
use std::fs::create_dir_all;
use std::{fs::exists, io::Result};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the command
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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

fn main() {
    let git_dir = ".ugit";
    let args = Args::parse();

    match args.command {
        Commands::Init => {
            init_repository(git_dir)
                .expect("Error occured when attempting to initialize repository");
        }
        Commands::Add { files } => {
            println!("Adding files: {:?}", files);
        }
        Commands::Commit { message } => {
            println!("Committing with message: {}", message);
        }
        Commands::Diff { file_a, file_b } => {
            println!("Diffing two files: {} | {}", file_a, file_b);
        }
    }
}

fn init_repository(dir: &str) -> Result<()> {
    if exists(dir).is_ok() {
        println!("Repository already initialized");
        Ok(())
    } else {
        println!("Initializing repository {}...", dir);
        create_dir_all(format!("{}/objects", dir))
    }
}
