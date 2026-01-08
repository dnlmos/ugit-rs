mod base;
mod cli;
mod data;

use clap::Parser;
use cli::{Args, Commands, init_repository};

use crate::{
    base::{checkout, create_tag, get_oid, log},
    cli::Config,
    data::get_ref,
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
                None => match get_ref("HEAD", &config) {
                    Ok(oid) => oid,
                    Err(e) => {
                        eprintln!("No OID provided and failed to fetch HEAD: {e}");
                        return;
                    }
                },
            };
            let oid = get_oid(&target_oid, &config);
            match log(&oid, &config) {
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
                None => match get_ref("HEAD", &config) {
                    Ok(oid) => oid,
                    Err(e) => {
                        eprintln!("No OID provided and failed to fetch HEAD: {e}");
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
            data::iter_refs(&config);
        }
    }
}
