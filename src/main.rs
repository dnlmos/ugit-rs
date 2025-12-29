mod base;
mod cli;
mod data;

use std::path::Path;

use clap::Parser;
use cli::{Args, Commands, init_repository};
use ugit_rs::cli::BASE_DIR;

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Init => {
            init_repository().expect("Error occured when attempting to initialize repository");
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
            println!("Writing tree");
            base::write_tree(Path::new(BASE_DIR));
        }
        Commands::ReadTree => {
            println!("Reading tree");
            base::read_tree("9ae035d0aef480ee04f5f7dc72ddb23e96749e63");
        }
    }
}
