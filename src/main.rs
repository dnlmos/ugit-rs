use clap::{Parser, Subcommand};

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
    let args = Args::parse();

    match args.command {
        Commands::Init => {
            println!("Initializing repository...");
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
