# caps

> **CAPS** (Classroom Achievement Point System) is a simple, cross-platform graphical tool that rewards consistent typing on Typing.com with points on ClassDojo

---

## Table of Contents
- [Why caps?](#why-caps)
- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [Example Workflow (Fish Shell)](#example-workflow-fish-shell)
- [Contributing](#contributing)
- [License](#license)

---

## Why caps?

- **Focus on effort:** Reward students for sustained practice rather than speed alone.
- **Hands‑off for teachers:** Once set up, caps can pull typing data from typing.com and add points to a student's profile in a ClassDojo class.
- **Built with Rust:** Fast, memory‑safe, and easy to distribute as a single binary.

---

## Features

- ✅ **5‑minute detection** – tracks each student’s continuous typing session via the Typing.com API.
- ✅ **Automatic point award** – posts a "+1" point to the classroom leaderboard when the threshold is met.
- ✅ **Zero‑config start** – sensible defaults; optional TOML/JSON config for API keys, classroom ID, point value, and polling interval.
- ✅ **Cross‑platform** – works on Windows, macOS, and Linux (single‑file binary).

---

## Installation

Download a pre-built binary from the [GitHub releases page](https://github.com/yourusername/caps/releases)

---

## Configuration

Caps will look for a file named `config.toml` following locations:

| Location | Priority |
|----------|----------|
| `./config.toml` (working directory) | 1 |
| `$XDG_CONFIG_HOME/caps/config.toml` (Linux/macOS) | 2 |
| `%APPDATA%\caps\config.toml` (Windows) | 2 |

If none can be found, it will automatically generate one in either `$XDG_CONFIG_HOME/caps/` or `%APPDATA%\caps\`.

---

## Contributing

Contributions are welcome! Please:

1. Fork the repo.  
2. Create a feature branch (`git checkout -b feat/your‑feature`).  
3. Ensure the code builds (`cargo test`).  
4. Open a Pull Request with a clear description.

---

## License

GPL-3.0 © 2025 Brayden Chan. See `LICENSE` for details.