# raur

An AUR helper and Pacman wrapper written in Rust.

## Features

* Search for packages in the Arch Linux repositories and the AUR.
* Colored output for better readability.
* Shows which packages are already installed.

## Installation

```bash
cargo install raur
```

## Usage

### Search for a package

```bash
raur search <query>
```

## Building from source

```bash
git clone https://github.com/user/raur.git
cd raur
cargo build --release
```
