<p align="center">
  <img src="assets/Text-README.jpg" alt="InEverything" width="720">
</p>

# InEverything

Everything-like, instantly launching, ultra fast file search and management app — written in Rust with
[eframe/egui](https://github.com/emilk/egui). The interface is bilingual (Türkçe / English) and the
language switches instantly, without a restart.

![InEverything](assets/InEverything-App.png)

## Features

- **Persistent index** — files are scanned in parallel once and written to a single `mmap`-ed file, so
  every later launch takes milliseconds
- **Parallel system scan** (`rayon` + work queue); scan roots and the exclude list come from the config
- **Allocation-free, lock-free search** — raw byte search with `memchr::memmem`, 0-3 scoring, best N
  results picked with `select_nth_unstable`
- **Live watch** (`notify`) — created and deleted files show up immediately; the index refreshes itself
  automatically after 20k changes
- **Extension filter** (`*.pdf`) and combined queries (`*.pdf report`)
- **Virtualized result list** — 10,000 rows stay smooth by default
- **Right-click menu** — open, open file location, copy path, copy file to clipboard, move
- **Row buttons** — `COPY PATH` / `COPY FILE` / `MOVE`: copy the path or the file itself to the
  clipboard, or move the file to another folder
- **Keyboard driven** — `Ctrl+I` search, `F5` rescan, `↑↓`/`Home`/`End` navigation, `F1` copy path,
  `F2` copy file, `F3`/`Enter` move
- **Turkish / English interface** — the round button in the top-right corner switches language on the
  spot (`EN` while Turkish, `TR` while English); the choice is remembered across restarts
- **Futuristic neon theme**, embedded logo, Windows / macOS / Linux support

## Installation

```bash
git clone https://github.com/colgecen/InEverything.git
cd InEverything
cargo build --release
```

Prebuilt binaries for all three platforms are available on the
[releases](https://github.com/colgecen/InEverything/releases) page.

## Building

`./ineverything` is an interactive build helper. Run it with no arguments and it builds the project,
then asks which target system and package format you want:

```bash
./ineverything                  # build, then interactive menu (OS -> format -> package)
./ineverything --list           # show every available target
./ineverything --install        # symlink it into ~/.local/bin to call it from anywhere
./ineverything linux appimage   # build directly, without the menu
```

| OS | Formats |
| --- | --- |
| Linux | `appimage` · `rpm` · `targz` · `binary` |
| Windows | `exe` · `exe-zip` |
| macOS | `app` · `targz` · `binary` |

Everything lands in `build/`. InEverything is built on eframe/egui + winit, so its window backend
needs the target system's SDK: Windows and macOS packages have to be built on that OS (or with the
bundled GitHub Actions workflow). Cross-compiling to Windows is attempted when `cargo-xwin` or
`mingw-w64` is installed; the script explains what is missing otherwise.

The bare Cargo commands still work as usual:

```bash
cargo build --release      # target/release/ineverything
cargo install --path .     # installs `ineverything` into ~/.cargo/bin
```

## Usage

```bash
./target/release/ineverything
```

Start typing in the search box — results filter as you type and the first row is selected
automatically. **Double-click** a row to open the file; the right-click menu offers `Open`,
`Open file location`, `Copy path`, `Copy file to clipboard` and `Move…`. The row buttons do the same
thing without the mouse menu, and every action has a keyboard shortcut.

### Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+I` | Focus the search box |
| `F5` | Rescan the filesystem |
| `↑` `↓` | Previous / next row |
| `Home` `End` | First / last row |
| `F1` · `Ctrl+1` | Copy the path to the clipboard |
| `F2` · `Ctrl+2` | Copy the file to the clipboard |
| `F3` · `Enter` · `Ctrl+3` | Move the file (pick the destination in the file manager) |

> On some platforms `Ctrl+C` / `Ctrl+Shift+C` are consumed by the windowing system as *copy*, so `F1`,
> `F2`, `F3` (and `Ctrl+1/2/3`) are the reliable shortcuts.

### Language

The round button in the top-right corner of the window switches the whole interface — labels, column
headers, menus, status messages and even units (`sn` / `s`). Nothing is reloaded or rebuilt; every
string is read from the selected language on each frame. The choice is stored as `dil` (`"tr"` or
`"en"`) in the config file.

## Configuration

| Path | Contents |
| --- | --- |
| `~/.config/InEverything/config.json` | Settings, including the interface language |
| `~/.local/share/InEverything/indeks.bin` | The persistent file index |

Scan roots and excludes are configurable in `config.json` (`kokler`, `haric`); leaving the roots empty
scans every mounted volume.

## License

Released under the [MIT](LICENSE) license.
