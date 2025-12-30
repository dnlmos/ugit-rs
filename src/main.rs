mod base;
mod cli;
mod data;

use clap::Parser;
use cli::{Args, Commands, init_repository};

use crate::cli::Config;

fn main() {
    let args = Args::parse();

    let config = Config {
        base_dir: std::path::PathBuf::from("."),
        git_dir: std::path::PathBuf::from(".").join(".ugit"),
    };

    match args.command {
        Commands::Init => match init_repository(&config) {
            Ok(_) => println!("Repository initialized successfully."),
            Err(e) => eprintln!(
                "Error occurred when attempting to initialize repository: {}",
                e
            ),
        },
        Commands::Add { files } => {
            println!("Adding files: {:?}", files);
        }
        Commands::Commit { message } => {
            println!("Committing with message: {}", message);
        }
        Commands::Diff { file_a, file_b } => {
            println!("Diffing two files: {} | {}", file_a, file_b);
        }
        Commands::WriteTree => {
            println!("Writing tree");
            match base::write_tree(&config.base_dir, &config) {
                Ok(_) => println!("Writing tree successfully."),
                Err(e) => eprintln!("Error occured when attempting to write tree: {}", e),
            }
        }
        Commands::ReadTree { tree_oid } => {
            println!("Reading tree");
            match base::read_tree(&tree_oid, &config) {
                Ok(_) => println!("Reading tree successfully."),
                Err(e) => eprintln!("Error occured when attempting to read tree: {}", e),
            }
        }
    }
}
