<h1 align="center">InEverything</h1>

<p align="center">An Everything-inspired file search and management app with a bilingual (Türkçe / English) interface that runs on your desktop (Rust + eframe/egui).</p>

<p align="center">
  <img src="assets/InEverything.gif" alt="InEverything demo" width="700">
</p>

## Features

- Persistent index: files are scanned in parallel once and written to a single `mmap`-ed file, so later launches take milliseconds
- Allocation-free, lock-free search (`memchr::memmem`) with extension filters (`*.pdf`) and combined queries (`*.pdf report`)
- Live watch (`notify`): created and deleted files show up instantly, the index refreshes itself after 20k changes
- Virtualized result list, right-click menu and per-row buttons: `COPY PATH` / `COPY FILE` / `MOVE`
- Keyboard shortcuts: `Ctrl+I` search, `F5` rescan, `↑↓` navigate, `F1` copy path, `F2` copy file, `F3`/`Enter` move
- 2 languages: Turkish and English, switched instantly from the round button in the top-right corner
- Futuristic neon theme with embedded logo, Windows / macOS / Linux support

## Installation

Build and run the project locally:

```bash
git clone https://github.com/colgecen/InEverything.git
cd InEverything
cargo build --release
```

To produce packages with the menu-driven build tool:

```bash
./ineverything --install
ineverything
```

Pick a target and format (Linux: AppImage/RPM/tar.gz/binary, Windows: .exe/.exe+zip, macOS: app/tar.gz/binary).
Prebuilt binaries are on the [releases](https://github.com/colgecen/InEverything/releases) page.

## Usage

Run it with:

```bash
./target/release/ineverything
```

Start typing in the search box — results filter as you type and the first row is selected automatically. Double-click a row to open the file, use the right-click menu or the row buttons to copy the path, copy the file to the clipboard or move it. The round button in the top-right corner reads `EN` while the interface is Turkish and `TR` while it is English; every label, menu, status message and unit switches in place without a restart.

## Screenshots

<p align="center">
  <img src="assets/InEverything-App.png" alt="InEverything main window" width="900">
</p>

## License

This project is licensed under the [MIT](LICENSE) license.