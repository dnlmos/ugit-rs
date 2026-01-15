mod base;
mod cli;
mod data;
mod utils;

use crate::base::init_repository;
use clap::Parser;
use cli::{Args, Commands};

use crate::{
    base::{checkout, create_branch, create_tag, get_oid, k, log},
    cli::Config,
    data::{Follow, get_ref},
};

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
        Commands::Commit { message } => match base::create_commit(message, &config) {
            Ok(_) => println!("Writing commit successfully."),
            Err(e) => eprintln!("Error occured when attempting to write commit: {}", e),
        },
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
            let oid = get_oid(&tree_oid, &config);
            match base::read_tree(&oid, &config) {
                Ok(_) => println!("Reading tree successfully."),
                Err(e) => eprintln!("Error occured when attempting to read tree: {}", e),
            }
        }
        Commands::Log { oid } => {
            let target_oid = match oid {
                Some(id) => id,
                None => match get_ref("HEAD", &Follow::IfSymbolic, &config) {
                    Ok(Some(oid)) => oid.to_string(),
                    Ok(None) => {
                        eprintln!("No OID provided and HEAD does not exist");
                        return;
                    }
                    Err(e) => {
                        eprintln!("No OID provided and failed to fetch HEAD (@): {e}");
                        return;
                    }
                },
            };
            get_oid(&target_oid, &config);
            match log(&target_oid, &config) {
                Ok(history) => println!("{history}"),
                Err(e) => eprintln!("Error occured when attempting to running log: {}", e),
            };
        }
        Commands::Checkout { oid } => {
            let oid = get_oid(&oid, &config);
            match checkout(&oid, &config) {
                Ok(_) => println!("Switched to {}", oid),
                Err(e) => eprintln!("Error occured when attempting to checkout {}: {}", oid, e),
            };
        }
        Commands::Tag { name, oid } => {
            let target_oid = match oid {
                Some(id) => id,
                None => match get_ref("HEAD", &Follow::IfSymbolic, &config) {
                    Ok(Some(ref_)) => ref_.to_string(),
                    Ok(None) => {
                        eprintln!("No OID provided and HEAD does not exist");
                        return;
                    }
                    Err(e) => {
                        eprintln!("No OID provided and failed to fetch HEAD (@): {e}");
                        return;
                    }
                },
            };

            match create_tag(&name, &target_oid, &config) {
                Ok(_) => println!("Tag '{}' created successfully, oid '{}'", &name, target_oid),
                Err(e) => eprintln!(
                    "Error occured when attempting to create tag {}: {}",
                    name, e
                ),
            }
        }
        Commands::K => {
            let _ = k(&config);
        }
        Commands::Branch { name, start_point } => {
            let start = start_point.unwrap_or_else(|| "@".to_string());
            match create_branch(&name, &start, &config) {
                Ok(_) => println!(
                    "Branch '{}' with starting point {} created successfully",
                    name, start
                ),
                Err(e) => eprintln!(
                    "Error occured when attempting to create branch {} at starting point {}: {}",
                    name, start, e
                ),
            }
        }
    }
}
