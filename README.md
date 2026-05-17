# Keystroke

Keystroke is a simple, cross-platform graphical tool that rewards consistent typing with points

---

## Why Keystroke?

- **Focus on effort:** Reward students for sustained practice rather than speed alone.
- **Hands‑off for teachers:** Once set up, Keystroke can pull typing data from Typing.com or TypingClub and add points to a student's profile in a ClassDojo or Perkido class.
- **Built with Rust:** Fast, memory‑safe, and easy to distribute as a single binary.

---

## Features

- ✅ **5‑minute detection** – tracks each student’s continuous typing session via the Typing.com or TypingClub API.
- ✅ **Automatic point award** – posts a "+1" point to the classroom leaderboard when the threshold is met.
- ✅ **Zero‑config start** – sensible defaults; optional TOML config, classroom ID, point value, and polling interval.
- ✅ **Cross‑platform** – works on Windows, macOS, and Linux (single‑file binary).

---

## Installation

Download a pre-built binary from the [GitHub releases page](https://github.com/megabyte6/keystroke/releases)

---

## Configuration

Keystroke will look for a file named `settings.toml` following locations:

| Location | Priority |
|----------|----------|
| `./settings.toml` (working directory) | 1 |
| `$XDG_CONFIG_HOME/keystroke/settings.toml` (Linux) | 2 |
| `~/Library/Application Support/keystroke/settings.toml` (macOS) | 2 |
| `%APPDATA%\keystroke\settings.toml` (Windows) | 2 |

If none can be found, it will automatically generate one in either `$XDG_CONFIG_HOME/keystroke/`, `~/Library/Application Support/keystroke/`, or `%APPDATA%\keystroke\`.

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
