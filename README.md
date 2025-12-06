# raur

**Repo Unified Helper** - A simple AUR helper I wrote in Rust 🦀.

I built this because I wanted to learn Rust and I kept breaking my system with other helpers. It supports the usual `pacman` operations and handles AUR packages reasonably well (I think).

It "just works" somehow. Please don't judge the code too hard.

## Features

I added a few things that I personally find useful (and that I could figure out how to code):

*   **Arch News Integration**: It checks the Arch Linux News before upgrading so I don't accidentally break my system (again). This has saved me at least twice.
*   **Fast-ish**: It uses `libalpm` directly because spawning `pacman` for everything felt slow, and I wanted to feel like a real Rust developer.
*   **Interactive Menu**: A simple menu to pick packages, because I can never remember if it's `google-chrome` or `google-chrome-stable`.
*   **Safety**: Prompts to review `PKGBUILD`s and view `git diff`s. I usually just pretend to read them, but it's good to have the option.
*   **Configurable**: I got tired of passing flags every time, so there's a config file now.

## Installation

If you want to try it out (at your own risk):

```bash
git clone https://github.com/Manpreet113/raur.git
cd raur
cargo install --path .
```

## Usage

It mostly behaves like other helpers, so you don't have to learn new commands.

### Search & Install
```bash
raur <query>
# Example: raur spotify
```

### Install Specific Package
```bash
raur -S <package_name>
```

### System Upgrade
Updates everything (Repo + AUR) and shows the news.
```bash
raur -Syu
```

## Configuration

You can tweak it at `~/.config/raur/config.toml`.

**My Config:**

```toml
# ~/.config/raur/config.toml

build_dir = "/home/user/.cache/raur"
editor = "nvim" # or nano if you're normal
clean_build = false
show_news = true
diff_viewer = true
```

## Might work on later

Things I might add if I ever figure out how they work:

*   **Parallel Downloads**: Downloading multiple packages at once (if I can stop the race conditions).
*   **Split Packages**: Right now it builds everything, sorry.
*   **Advanced Dependency Solving**: "Providers" are hard.
*   **Local PKGBUILDs**: Maybe one day.

## License

MIT (Do whatever you want with it)
