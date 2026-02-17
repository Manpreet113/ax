use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ax")]
#[command(about = "Repo Unified Helper", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Search query
    pub query: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(short_flag = 'S')]
    Sync {
        #[arg(short = 'y', long)]
        refresh: bool, // -y
        #[arg(short = 'u', long)]
        sysupgrade: bool, // -u
        #[arg(long)]
        cleanbuild: bool,
        packages: Vec<String>,
    },
    #[command(short_flag = 'R')]
    Remove {
        packages: Vec<String>,
    },
}
