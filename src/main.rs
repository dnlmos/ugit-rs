mod base;
mod cli;
mod data;

use std::path::Path;

use clap::Parser;
use cli::{Args, Commands, GIT_DIR, init_repository};

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Init => {
            init_repository(GIT_DIR)
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
        Commands::WriteTree => {
            println!("writing tree");
            base::write_tree(Path::new("."));
        }
    }
}
