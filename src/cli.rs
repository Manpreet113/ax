use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ax", version)]
#[command(about = "A complete, fast, and unified pacman wrapper and AUR helper", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Search query
    #[arg(trailing_var_arg = true)]
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

        /// Packages to install
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        packages: Vec<String>,
    },
    #[command(short_flag = 'R')]
    Remove {
        /// Packages to remove
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        packages: Vec<String>,
    },
    #[command(short_flag = 'Q')]
    Query {
        /// Arguments to pass to pacman -Q
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(short_flag = 'D')]
    Database {
        /// Arguments to pass to pacman -D
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(short_flag = 'F')]
    Files {
        /// Arguments to pass to pacman -F
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(short_flag = 'T')]
    Deptest {
        /// Arguments to pass to pacman -T
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    #[command(short_flag = 'U')]
    Upgrade {
        /// Arguments to pass to pacman -U
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
