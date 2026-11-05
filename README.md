# glzr

A lightweight, terminal-first developer workspace for Linux: terminal, code editor, git panel, and localhost preview in a single unified interface. Zero AI bloat, zero plugins, zero LSP overhead.

```
+-----------------------------------------------------------------------+
| [Alt+1: web] [Alt+2: backend]                     Dark | Branch: main |
+-----------------------------------+-----------------------------------+
| FILES | GIT | SEARCH              | TERMINAL 1                        |
| src/                              | $ cargo check                     |
|   lib.rs                          |    Checking glzr v0.1.0           |
|   fs/mod.rs                       |    Finished `dev` profile         |
|                                   +-----------------------------------+
| EDITOR: src/main.rs               | PREVIEW: http://localhost:5173    |
| fn main() {                       | [ glzr UI Live Preview ]          |
|     glzr::run();                  |                                   |
| }                                 |                                   |
+-----------------------------------+-----------------------------------+
```

## Why glzr?

Modern development increasingly revolves around terminal workflows, CLI utilities, and AI coding agents (such as Claude Code, Aider, and Antigravity). However, traditional IDEs consume gigabytes of memory running language servers, telemetry, extension hosts, and background indexing services.

glzr takes a different approach:
- **Terminal-First:** Terminals are first-class citizens. AI agents and CLI tools run directly inside native PTY sessions rather than through restrictive GUI abstractions.
- **Ultra-Lightweight:** Written with a Rust (Tauri 2) backend and WebKitGTK 4.1, glzr maintains a baseline memory footprint of approximately 110-140 MB PSS.
- **Unified Layout Model:** Everything on the screen is a tab inside a pane. Panes can be split horizontally or vertically, maximized, or distributed across up to 9 workspaces.
- **Code Review Ready:** Built-in hunk-by-hunk visual diffing and streaming ripgrep search make reviewing agent generated code fast and reliable.
- **Linux Focused:** Optimized specifically for Linux desktop environments (Ubuntu 24.04, Zorin OS 18, Debian, and compatible distributions).

---

## Core Capabilities

1. **Integrated Terminal**
   - Multi-tab and split-pane terminal emulator powered by `xterm.js` with WebGL hardware acceleration.
   - True 24-bit color support and interactive login shells via `portable-pty`.
   - In-terminal search (`Alt+S`), smart copy/paste handling, and instant resize synchronization.

2. **Focused Code Editor**
   - High-performance editor powered by CodeMirror 6 with syntax highlighting for JavaScript, TypeScript, Rust, Go, Python, Svelte, Markdown, HTML, CSS, TOML, YAML, and JSON.
   - Atomic file saving that preserves existing file permissions, line endings (LF/CRLF), and UTF-8 byte order marks.
   - Automatic background file-watching that prompts or reloads when files change on disk.

3. **Visual Git Panel**
   - Comprehensive working tree status tracking staged, unstaged, and untracked files.
   - Interactive hunk-level visual diffing powered by `@codemirror/merge`.
   - Stage, unstage, and discard operations at both individual file and single-hunk granularity.
   - Local commit authoring, branch checkout, and remote fetch/pull/push operations.

4. **Fast Project-Wide Search**
   - Sidecar-powered `ripgrep` (`rg`) search with streaming match delivery.
   - Instant cancellation upon query modification, regex search support, and automatic snippet length truncation.

5. **Isolated Localhost Preview**
   - Sandboxed iframe browser tab tailored for testing local web development servers (`localhost` and `127.0.0.1`).
   - Navigation controls, URL address bar, refresh button, external browser handoff, and automatic suspension for inactive background tabs.

6. **Flexible Tiling & Workspaces**
   - Binary tree split layout supporting recursive horizontal and vertical divisions.
   - Up to 9 independent workspaces accessible via `Alt+1` through `Alt+9`.
   - Automatic session state persistence: window dimensions, open folders, pane arrangements, and active tabs restore seamlessly on restart.

7. **Hardware Acceleration & Battery Efficiency**
   - Native WebKitGTK DMA-BUF hardware sharing with integrated GPUs (such as Intel Iris Xe).
   - Discrete GPU power management keeping high-drain cards (such as NVIDIA RTX) in D3cold sleep during standard editing.
   - Kinetic smooth scrolling with requestAnimationFrame lerp interpolation and key repeat throttling to eliminate stutter and input overrun.

---

## Resource Usage and Performance

### Measured Benchmarks

| Metric / Component                                     | Target     | Measured Result            |
| ------------------------------------------------------ | ---------- | -------------------------- |
| Release Binary Size (`glzr`)                           | < 15 MB    | 3.4 MB - 9.6 MB            |
| Frontend Bundle (compressed JS/HTML/CSS)               | < 300 KB   | ~80 KB                     |
| Total Packaged Payload (including sidecar and fonts)   | 12-25 MB   | ~11.7 MB (uncompressed)    |
| Debian Package (`.deb`)                                | 12-25 MB   | ~8-15 MB (compressed)      |
| AppImage Bundle                                        | 15-30 MB   | ~15-25 MB (compressed)     |
| Baseline RAM (1 pane, 1 active terminal)               | 80-150 MB  | ~110-140 MB PSS total      |
| Typical Daily Session (3-5 mixed editor/git/term tabs) | 150-250 MB | ~180-230 MB PSS total      |

### How to Measure RAM Accurately on Linux

WebKitGTK on Linux separates runtime execution across multiple processes:
- Main application process (`glzr`)
- WebKit rendering engine (`WebKitWebProcess`)
- WebKit network manager (`WebKitNetworkProcess`)

To measure true physical RAM usage without double-counting shared libraries, use the Proportional Set Size (PSS) metric:

```bash
# Using smem (recommended)
smem -P glzr -c "pid user command pss"

# Or using a one-line shell script to sum PSS across all glzr processes
cat /proc/$(pgrep -d ' ' -f 'glzr|WebKitWebProcess|WebKitNetworkProcess')/smaps 2>/dev/null | grep -i '^Pss:' | awk '{sum+=$2} END {print sum/1024, "MB"}'
```

---

## Comparison with Other Editors

| Aspect               | Zed                                                               | VS Code                                                            | glzr                                                                    |
| -------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------- |
| Philosophy           | Editor-first with integrated terminal                             | All-in-one extensible IDE                                          | Terminal-first with built-in lightweight editor                         |
| Rendering Engine     | Custom GPU engine (GPUI)                                          | Chromium / Electron                                                | Tauri 2 + Native System WebKitGTK 4.1                                   |
| Language Support     | Language Server Protocol (LSP), inline AI, treesitter             | Full LSP ecosystem, extensions                                     | CodeMirror 6 syntax highlight, zero LSP, zero internal AI               |
| Web Preview          | Requires external browser                                         | Requires extension or external browser                             | Built-in sandboxed localhost preview iframe                             |
| CLI Agent Workflow   | Secondary terminal panel                                          | Secondary terminal panel                                           | Primary first-class citizen (CLI agents run directly in full terminals) |
| Target Platform      | Multi-platform                                                    | Multi-platform                                                     | Linux native focused (Ubuntu 24.04, Zorin OS 18, Debian)                |

---

## Installation

### System Prerequisites

glzr requires `git` and WebKitGTK 4.1 installed on your Linux system:

```bash
sudo apt update && sudo apt install -y git libwebkit2gtk-4.1-0
```

### Option 1: Debian Package (`.deb`)

Download the `.deb` file from the GitHub Releases page and install it:

```bash
sudo dpkg -i glzr_*_amd64.deb
sudo apt-get install -f # Fix any missing system dependencies if needed
```

The Debian package automatically places `glzr` into your `$PATH` and registers desktop launchers. You can open any directory directly from your terminal:

```bash
glzr .
```

### Option 2: AppImage

Download the `.AppImage` file from the GitHub Releases page, make it executable, and run:

```bash
chmod +x glzr_*_amd64.AppImage
./glzr_*_amd64.AppImage
```

To run `glzr` from any terminal path using AppImage, create a symlink in your local bin directory:

```bash
mkdir -p ~/.local/bin
ln -sf "$(pwd)/glzr_amd64.AppImage" ~/.local/bin/glzr
```

Ensure `~/.local/bin` is included in your `$PATH` environment variable.

---

## Keyboard Shortcuts

All application and window management shortcuts in glzr utilize the `Alt` key to prevent interference with standard terminal PTY signals (such as `Ctrl+C`, `Ctrl+Z`, or `Ctrl+D`).

### Global Application Shortcuts (Active anywhere, including inside the terminal)

| Action                           | Shortcut                                      |
| -------------------------------- | --------------------------------------------- |
| Command Palette                  | `Alt+P`                                       |
| Quick Open File                  | `Alt+O`                                       |
| File Tree Sidebar                | `Alt+E`                                       |
| Git Sidebar                      | `Alt+V`                                       |
| Search Sidebar                   | `Alt+G`                                       |
| New Terminal in Focused Pane     | `Alt+T`                                       |
| Close Active Tab                 | `Alt+W`                                       |
| Split Pane Right                 | `Alt+R`                                       |
| Split Pane Down                  | `Alt+J`                                       |
| Close Focused Pane               | `Alt+Q`                                       |
| Maximize / Restore Focused Pane  | `Alt+M`                                       |
| Previous Tab in Focused Pane     | `Alt+[`                                       |
| Next Tab in Focused Pane         | `Alt+]`                                       |
| Move Focus Between Panes         | `Alt+Left`, `Alt+Right`, `Alt+Up`, `Alt+Down` |
| Switch to Workspace 1-9          | `Alt+1` .. `Alt+9`                            |
| Zoom Font In / Out / Reset       | `Alt+=` / `Alt+-` / `Alt+0`                   |

### Code Editor Shortcuts (Active when CodeMirror editor is focused)

| Action                   | Shortcut            |
| ------------------------ | ------------------- |
| Save Changes             | `Ctrl+S`            |
| Undo / Redo              | `Ctrl+Z` / `Ctrl+Y` |
| Find and Replace in File | `Ctrl+F`            |
| Go to Line               | `Ctrl+G`            |
| Toggle Line Comment      | `Ctrl+/`            |
| Select Next Occurrence   | `Ctrl+D`            |

### Terminal and Panel Shortcuts

| Context   | Action                     | Shortcut                              |
| --------- | -------------------------- | ------------------------------------- |
| Terminal  | Copy Selected Text         | `Ctrl+C` (only when text is selected) |
| Terminal  | Paste from Clipboard       | `Ctrl+Shift+V` or middle mouse click  |
| Terminal  | Find in Terminal Scrollback| `Alt+S`                               |
| Preview   | Focus Address Bar          | `Ctrl+L`                              |
| Preview   | Reload Page                | `Ctrl+R` or `F5`                      |
| Git Panel | Submit Commit              | `Ctrl+Enter` inside commit text input |

---

## Configuration

glzr stores its user configuration in `~/.config/glzr/config.toml`. The configuration is automatically created with sensible defaults on the first launch.

Example `config.toml`:

```toml
[appearance]
theme = "dark"
font_family = "system-mono"
font_size = 14

[editor]
tab_size = 2
insert_spaces = true
word_wrap = false
line_numbers = true
minimap = false
bracket_matching = true
highlight_active_line = true

[terminal]
font_family = "system-mono"
font_size = 14
cursor_style = "block"
cursor_blink = false
scrollback_lines = 10000

[files]
ignore = [".git", "node_modules", "target", "dist", "build", ".svelte-kit"]

[git]
command_timeout_seconds = 10
diff_size_limit_mb = 2

[search]
max_results = 2000
snippet_max_chars = 300
debounce_ms = 200

[preview]
allowed_hosts = ["localhost", "127.0.0.1"]
auto_detect_port = false

[session]
restore = true
recent_folders_limit = 5

[logging]
level = "info"

[keybindings]
command_palette = "Alt+P"
quick_open = "Alt+O"
sidebar_files = "Alt+E"
sidebar_git = "Alt+V"
sidebar_search = "Alt+G"
new_terminal = "Alt+T"
close_tab = "Alt+W"
split_right = "Alt+R"
split_down = "Alt+J"
close_pane = "Alt+Q"
maximize_pane = "Alt+M"
terminal_find = "Alt+S"
prev_tab = "Alt+["
next_tab = "Alt+]"
```

---

## Building from Source

### 1. Install System Dependencies (Ubuntu 24.04 / Zorin OS 18 / Debian)

```bash
sudo apt update && sudo apt install -y \
  build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev git
```

### 2. Install Rust and Node.js

Ensure Rust (1.80+ via rustup) and Node.js 22 LTS are installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs
```

### 3. Clone Repository and Install Frontend Dependencies

```bash
git clone https://github.com/Fahmi-mi/glzr.git
cd glzr
npm install
```

### 4. Run Development Server

```bash
npm run tauri dev
```

### 5. Build Production Packages

```bash
npm run tauri build
```

The compiled binaries and installers will be generated under:
- `src-tauri/target/release/glzr` (direct executable)
- `src-tauri/target/release/bundle/deb/` (Debian package)
- `src-tauri/target/release/bundle/appimage/` (AppImage bundle)

---

## Intentional Non-Goals

To maintain speed, low memory usage, and architectural simplicity, the following features are intentionally out of scope:

- **No Language Server Protocol (LSP):** No background language server processes consuming memory and CPU cycles.
- **No Plugin or Extension System:** No third-party plugin runtime or foreign sandbox overhead.
- **No Built-In AI Assistant:** AI models and CLI agents run directly inside the native terminal according to user preference.
- **No Graphical Debugger:** Interactive debugging is conducted via CLI debuggers (`gdb`, `lldb`, `dlv`) in the terminal.
- **No Global Search and Replace:** Search is read-only via `ripgrep`; bulk modifications across the codebase are left to explicit tooling.
- **Linux Exclusive:** Tailored exclusively to the Linux desktop ecosystem with WebKitGTK 4.1.

---

## Known Issues

A detailed list of identified constraints and platform considerations is maintained in [KNOWN_ISSUES.md](./KNOWN_ISSUES.md).

---

## License

This project is licensed under the [MIT License](./LICENSE).

Third-party component licenses and acknowledgments (ripgrep, xterm.js, CodeMirror 6, JetBrains Mono Nerd Font, etc.) are documented in [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).
