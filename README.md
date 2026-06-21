# 🍺 Beer Mouse Clicker (酒要点点)

<p align="center">
  <img src="icon.ico" width="128" alt="icon" />
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/rust-1.70%2B-orange.svg" /></a>
  <a href="https://github.com/cjxpj/beer_mouse_clicker/releases"><img src="https://img.shields.io/badge/platform-Windows%2010%2F11-brightgreen.svg" /></a>
  <a href="https://github.com/cjxpj/beer_mouse_clicker/releases"><img src="https://img.shields.io/github/v/release/cjxpj/beer_mouse_clicker" /></a>
</p>

<p align="center">
  <a href="README_EN.md">🇨🇳 中文</a>
</p>

**Beer Mouse Clicker** is a Windows mouse/keyboard macro automation tool built with Rust + egui. It provides visual orchestration of **22 task action types** — including mouse clicks, keystrokes, delays, image matching, scrolling, and more. Record, loop, and run in the background to automate repetitive desktop operations.

---

## ✨ Features

- **Visual Task Orchestration** — Add, edit, delete, and drag-to-reorder task steps with smooth animations
- **22 Task Action Types** — Covering mouse, keyboard, delays, image matching, scrolling, notifications, program launching, and more
- **Record & Replay** — Capture clicks, moves, scrolls, and keystrokes via a low-level mouse hook with auto-detection of long-press and key combos
- **Hotkey Control** — Customizable hotkey (default `F6`) triggers task start/stop from any window
- **Safety Locks** — Optionally lock keyboard input and confine mouse cursor during execution to prevent interference
- **System Tray** — Close to tray instead of quitting; right-click menu to show/exit; hotkey works in background
- **Auto-start** — Register in Windows startup via registry; separately configurable "run on launch" and "run on auto-start"
- **Image Recognition** — Screen template matching via NCC (Normalized Cross-Correlation) with configurable confidence threshold and window-scoped search
- **Bilingual (Chinese/English)** — Auto-detects system language, switchable at runtime
- **Catppuccin Mocha Dark Theme** — Elegant dark UI with automatic CJK font loading

---

## 📋 Task Action Types (22 Total)

| Action | Description |
|--------|-------------|
| Mouse Click | Click at specified coordinates |
| Mouse Down | Press mouse button at specified coordinates |
| Mouse Up | Release mouse button at specified coordinates |
| Mouse Move (Absolute) | Move mouse to absolute coordinates |
| Mouse Move (Relative) | Move mouse by relative offset |
| Mouse Move (Eased) | Smooth eased animation to target position |
| Mouse Move (Window Center) | Move mouse to the center of target window |
| Key Press | Press and release a single key |
| Combo Key | Press a key combination (e.g., `Ctrl+C`) |
| Key Down | Press a key without releasing |
| Key Up | Release a specified key |
| Mouse Wheel | Scroll at specified position |
| Delay | Wait for a fixed duration (ms) |
| Random Delay | Wait for a random duration within a range |
| Wait Until | Wait until a specific time of day |
| Image Match | Screen template matching, proceed on match |
| Wait Key | Wait for user to press a specified key |
| Wait Input | Wait for user text input (shows input dialog) |
| Notify | Show a notification message |
| Copy Text | Copy text to clipboard and paste |
| Open Program | Launch an external program |
| Show/Hide Window | Toggle main window visibility |

---

## 🎬 Screenshots

<!-- Screenshot placeholders: place images in repo root or docs/ and uncomment -->
<!-- ![Main Window](screenshots/main.png) -->
<!-- ![Task Edit](screenshots/edit.png) -->

---

## 📦 Installation

Download the latest `beer_mouse_clicker.exe` from the [Releases](https://github.com/cjxpj/beer_mouse_clicker/releases) page and run it — no installation required.

> 💡 On first run, `beer_clicker.bmc` (SQLite database) will be created next to the exe to store tasks, settings, and images.

### Auto-start

```bash
beer_mouse_clicker.exe --autostart
```

Registers itself under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` and starts minimized to tray on system boot.

---

## 🛠️ Build

### Prerequisites

- [Rust toolchain](https://rustup.rs) (MSVC target, 1.70+)
- Windows 10 / 11

### Compile

```bash
# Clone the repo
git clone https://github.com/cjxpj/beer_mouse_clicker.git
cd beer_mouse_clicker

# Ensure icon.ico exists in the project root

# Build release
cargo build --release
```

The executable will be at `target/release/beer_mouse_clicker.exe`.

---

## 🕹️ Usage

### Basic Operations

| Action | How |
|--------|-----|
| Add Task | Click the **+** button, choose a task type and configure |
| Edit Task | Click the **Edit** button on a task item |
| Delete Task | Click the **Delete** button on a task item |
| Reorder | Drag the handle on the left of a task item |
| Start | Press the hotkey (default `F6`) |
| Stop | Press the hotkey again |
| Record | Enable **Record**, then operate mouse/keyboard to generate tasks |

### Settings

| Setting | Default | Description |
|---------|---------|-------------|
| Task Interval | `1000ms` | Wait time between task cycles |
| Hotkey | `F6` | Hotkey to start/stop tasks |
| Lock Keyboard | Off | Block all keyboard input during execution |
| Lock Mouse | Off | Confine cursor to a 1×1 pixel area during execution |
| Loop Tasks | On | Whether to loop the task list |
| Background Mode | Off | Always run in background (close to tray) |
| Record Compression | On | Merge consecutive mouse move events |
| Auto-start | Off | Launch on system boot |
| Run on Launch | Off | Start executing tasks when the app opens |
| Run on Auto-start | Off | Start executing tasks when auto-started |

---

## 🏗️ Tech Stack

| Technology | Purpose |
|------------|---------|
| [Rust](https://www.rust-lang.org) | Language |
| [egui / eframe](https://github.com/emilk/egui) | GUI framework |
| [rusqlite](https://github.com/rusqlite/rusqlite) | SQLite data persistence |
| [winapi](https://github.com/retep998/winapi-rs) | Windows API (input injection, hooks, screenshots, registry) |
| [image](https://github.com/image-rs/image) | Image loading & thumbnails |
| [regex](https://github.com/rust-lang/regex) | Regex matching |
| [rand](https://github.com/rust-random/rand) | Random number generation |
| [rfd](https://github.com/PolyMeilex/rfd) | Native file dialogs |
| [winreg](https://github.com/gentoo90/winreg-rs) | Windows registry operations |

---

## 🤝 Contributing

Issues and Pull Requests are welcome.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## 👤 Author

- QQ: 2960965389
- GitHub: [@cjxpj](https://github.com/cjxpj)
- QQ Group: 310345976

---

## 📄 License

MIT License — see [LICENSE](LICENSE)
