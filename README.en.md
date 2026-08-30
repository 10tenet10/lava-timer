<p align="center">
  <img src="src-tauri/icons/readme-icon.png" width="96" height="96" alt="LavaTimer lava hourglass icon">
</p>

<h1 align="center">LavaTimer</h1>

<p align="center">
  <a href="README.md">简体中文</a> · English
</p>

<p align="center">
  A quiet, multi-project focus timer that lives on your desktop.<br>
  Start lightly, make steady progress, and see where your time went.
</p>

<p align="center">
  <a href="https://github.com/10tenet10/lava-timer/releases/latest"><strong>Download the latest release</strong></a>
  ·
  <a href="#local-development">Local development</a>
  ·
  <a href="LICENSE">MIT License</a>
</p>

<p align="center">
  <img alt="macOS 11+" src="https://img.shields.io/badge/macOS-11%2B-111111?style=flat-square&logo=apple">
  <img alt="Apple Silicon" src="https://img.shields.io/badge/architecture-Apple%20Silicon-333333?style=flat-square">
  <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square&logo=tauri&logoColor=white">
  <img alt="MIT License" src="https://img.shields.io/badge/license-MIT-FF7A2D?style=flat-square">
</p>

> **Project status: Feature-complete and in maintenance mode.** Future work will focus on bug fixes and compatibility maintenance rather than expanding the product scope.

## Three Views

LavaTimer adapts to the amount of information you need: keep it tucked into a corner of your desktop, expand it to check today's progress, or look back at how you spent your time.

### Capsule · Stay Focused

Shows only the active project, elapsed time, progress, and start button.

<p align="center">
  <img src="design-demos/readme-demo/assets/capsule.png" width="380" alt="LavaTimer capsule view">
</p>

### Focus Panel · See Today's Progress

The active project, daily goal, seven-day trend, and project progress come together in a draggable, translucent panel.

<p align="center">
  <img src="design-demos/readme-demo/assets/panel.png" width="380" alt="LavaTimer focus panel">
</p>

### History Overview · See Where Your Time Went

Switch between the last 7 days, the last 30 days, and all records. Review trends, project rankings, and the calendar heatmap, or select a project to focus on its time distribution.

<p align="center">
  <img src="design-demos/readme-demo/assets/overview.png" width="380" alt="LavaTimer history overview">
</p>

## Features

- Independent timers and daily goals for multiple projects
- Full panel, compact capsule, and 7-day / 30-day / all-time history views
- Current timer in the menu bar; click to show or hide the window
- Always-on-top desktop window, edge snapping, and automatic expansion direction
- Automatic pause when macOS locks, sleeps, turns off the display, or starts the screen saver
- Local timer state and history storage with no account or network connection required
- Crash-safe recovery that records time only up to the last heartbeat and reopens in a paused state

## Installation

1. Download the latest `.dmg` from [Releases](https://github.com/10tenet10/lava-timer/releases/latest).
2. Open the disk image and drag LavaTimer into Applications.
3. Launch LavaTimer from Applications. It will remain available in the menu bar.

The current prebuilt release targets **Apple Silicon (M1 or newer)** and requires macOS 11 or later. The release is ad-hoc signed and has not been notarized by Apple, so you may need to allow it under **System Settings → Privacy & Security** the first time you open it.

## Usage

1. Create a project and set its daily goal in Settings.
2. Start a timer from the capsule or full panel. Switching projects while a timer is running automatically records time for the previous project.
3. Change the time range in the history overview to review project time, trends, and streaks.

Closing the floating window only hides it; the timer continues running in the menu bar. After the app is quit or exits unexpectedly, it reopens in a paused state and does not count offline time as focus time.

## Data and Privacy

Project settings, daily timing data, and history are stored locally in the app WebView's local storage. LavaTimer does not create accounts, upload records, provide cloud sync, or create automatic backups.

Uninstalling the app or clearing its WebView data will delete your records. Make sure you no longer need them before doing so.

## Local Development

Requires Node.js 22+, stable Rust, and the macOS Xcode Command Line Tools.

```bash
npm install
npm run dev
```

Run all tests:

```bash
npm run check
```

Build the `.app` and `.dmg` bundles:

```bash
npm run build
```

Build artifacts are written to `src-tauri/target/release/bundle/`.

## Tech Stack

- [Tauri 2](https://v2.tauri.app/)
- Rust
- Vanilla HTML, CSS, and JavaScript

## License

[MIT](LICENSE) © 2026 10tenet10
