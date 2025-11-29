use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "raur")]
#[command(about= "An AUR helper and Pacman wrapper")]
pub struct Cli{
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands{
    Search{
        query: String,
    },
}