use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use std::env;
use std::io::Error;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut cmd = Cli::command();

    for shell in [Shell::Bash, Shell::Fish, Shell::Zsh] {
        generate_to(shell, &mut cmd, "ax", &out_dir)?;
    }

    println!("cargo:rerun-if-changed=src/cli.rs");
    Ok(())
}
