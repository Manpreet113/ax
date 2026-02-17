# Ax

Ax is a modern AUR helper and Pacman wrapper written in Rust, designed for simplicity, efficiency, and reliability. It aims to streamline package management on Arch Linux systems by seamlessly integrating official repository packages and the Arch User Repository (AUR).

## Features

- **Unified Interface**: Handles both official repository packages (via `pacman`) and AUR packages transparently.
- **Safety First**:
    - **Arch Linux News Integration**: Checks the latest Arch Linux News before performing system upgrades to prevent potential breakage.
    - **PKGBUILD Review**: Prompts users to review `PKGBUILD` files and view `git diff`s before building.
- **Improved Performance**: Utilizes `libalpm` directly for efficient package database queries, reducing the overhead of spawning `pacman` processes.
- **Interactive Search**: Simple and effective interactive menu for searching and selecting packages.
- **Configuration**: Highly configurable via a TOML configuration file to control build directories, editors, and behavior.
- **Clean Builds**: Easy option to force clean builds for troubleshooting.

## Installation

### Prerequisites

Ensure you have the base development tools installed:

```bash
sudo pacman -S --needed base-devel git
```

### Build from Source

Clone the repository and install using Cargo:

```bash
git clone https://github.com/Manpreet113/ax.git
cd ax
cargo install --path .
```

After installation, ensure `~/.cargo/bin` is in your `PATH`.

## Usage

Ax follows a syntax similar to `pacman` to minimize the learning curve.

### Search and Install
Search for a package in both official repositories and the AUR:

```bash
ax <query>
# Example: ax spotify
```

### Install Specific Package
Install a specific package by name:

```bash
ax -S <package_name>
```

### System Upgrade
Perform a full system upgrade (sync repo databases, upgrade repo packages, and upgrade AUR packages), checking for important news first:

```bash
ax -Syu
```

### Remove Package
Remove a package and its unused dependencies:

```bash
ax -R <package_name>
```

### Force Clean Build
To force a clean build (remove build directory before building):

```bash
ax -S <package> --cleanbuild
```

## Configuration

Ax can be configured via `~/.config/ax/config.toml`. The file is automatically created on first run if it doesn't exist.

### Example Configuration

```toml
# ~/.config/ax/config.toml

# Directory where AUR packages are built.
# Default: $XDG_CACHE_HOME/ax or ~/.cache/ax
build_dir = "/home/user/.cache/ax"

# Editor to use for PKGBUILD reviews.
# Default: $EDITOR or 'vi'
editor = "nvim"

# Whether to always clean the build directory before building.
# Default: false
clean_build = false

# Whether to check and display Arch Linux News before upgrades.
# Default: true
show_news = true

# Whether to use a diff viewer for inspecting changes.
# Default: true
diff_viewer = true
```

## License

MIT License. See [LICENSE](LICENSE) for details.
