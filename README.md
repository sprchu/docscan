# docscan

[中文文档](README_zh.md)

A terminal-based document search tool with a rich TUI.
Search keywords across **Word** (`.docx` / `.doc` / `.wps`),
**Excel** (`.xlsx` / `.xls` / `.et`), **PDF**, and **plain text** files —
all in parallel.

![main screenshot](https://pic1.imgdb.cn/item/69ff5ef78a9a8c274c5c255b.png)

## Features

- **Multi-format** — Word, Excel, PDF, plain text (`.txt`, `.md`, `.rs`,
  `.py`, `.js`, `.json`, `.csv`, `.log`, and more)
- **Multi-threaded** — adjustable parallelism powered by Rayon
- **Rich TUI** — built with Ratatui + Crossterm; keyboard-driven with
  mouse support
- **Browse directories** — visual directory picker for adding scan paths
- **Live filtering** — filter results by file type or path text
- **Command mode** — `:type`, `:filter`, `:dir`, `:threads`, and more
- **Clickable UI** — buttons, scrollable lists, type toggles all
  mouse-interactive
- **Extensible scanners** — add new file formats by implementing the
  `Scanner` trait

## Installation

```bash
cargo install --git https://github.com/user/docscan.git
```

## Usage

```bash
# Interactive TUI (scan current directory)
docscan

# With initial query
docscan -q "keyword"

# Search specific directories
docscan /path/to/docs /another/path

# Enable PDF support and set thread count
docscan --pdf -j 8 -q "budget" ~/Documents
```

### CLI options

| Flag     | Description                            |
| -------- | -------------------------------------- |
| `-q`     | Initial search keyword                 |
| `-j`     | Number of threads (default: CPU count) |
| `--pdf`  | Enable PDF scanning (off by default)   |
| `[dirs]` | One or more directories to scan        |

## Key bindings

### Normal mode

| Key            | Action                                          |
| -------------- | ----------------------------------------------- |
| `Tab`          | Cycle focus (Params → Paths → Results)          |
| `↑` / `↓`      | Navigate within focused panel                   |
| `←` / `→`      | Move cursor / adjust values                     |
| `Enter`        | Start scan (Config panels), open file (Results) |
| `:`            | Enter command mode                              |
| `Esc`          | Clear filter text                               |
| Character keys | Type in keyword / filter                        |

### Command mode (`:`)

| Command           | Action                                               |
| ----------------- | ---------------------------------------------------- |
| `:q`              | Quit                                                 |
| `:k <word>`       | Set search keyword                                   |
| `:j <n>`          | Set thread count                                     |
| `:type <name>`    | Toggle file type (`word` / `excel` / `pdf` / `text`) |
| `:filter <name>`  | Filter results by type (or `clear`)                  |
| `:dir add <path>` | Add scan directory                                   |
| `:dir rm [n]`     | Remove directory by index                            |
| `:dir browse`     | Open directory browser                               |
| `:dir clear`      | Clear all directories                                |

### Browse mode

| Key                   | Action                    |
| --------------------- | ------------------------- |
| `↑` / `↓` / `j` / `k` | Navigate directory list   |
| `Enter`               | Enter directory / confirm |
| `Backspace` / `←`     | Go to parent directory    |
| `Space`               | Confirm selection         |
| `~`                   | Jump to home directory    |
| `/`                   | Jump to filesystem root   |
| `Esc`                 | Cancel                    |

## Screenshots

### Browse directory popup

![Browse popup](https://pic1.imgdb.cn/item/69ff5ef68a9a8c274c5c255a.png)

### Scan results with filter

![Results](https://pic1.imgdb.cn/item/69ff5ee68a9a8c274c5c2557.png)

## Project structure

```
src/
├── main.rs          ─ entry point, CLI args, event loop
├── utils.rs         ─ misc helpers
├── command.rs       ─ :command execution
├── ui.rs            ─ terminal rendering (Ratatui)
│
├── app/             ─ application logic, split by concern
│   ├── types.rs     ─ data types (FileType, Focus, Mode, …)
│   ├── state.rs     ─ App struct + constructor
│   ├── config.rs    ─ query, threads, file type toggles
│   ├── results.rs   ─ result selection, filtering
│   ├── browse.rs    ─ directory browser
│   ├── cmd.rs       ─ command-mode text editing
│   ├── input.rs     ─ keyboard dispatch
│   └── mouse.rs     ─ mouse dispatch
│
└── scan/            ─ pluggable scanner architecture
    ├── mod.rs       ─ Scanner trait + orchestrator
    ├── docx.rs      ─ .docx scanner
    ├── doc.rs       ─ .doc / .wps binary scanner
    ├── excel.rs     ─ .xlsx / .xls / .et scanner
    ├── pdf.rs       ─ PDF scanner
    └── text.rs      ─ plain text scanner
```

## Dependencies

| Crate                                                  | Purpose                           |
| ------------------------------------------------------ | --------------------------------- |
| [Ratatui](https://ratatui.rs)                          | Terminal UI framework             |
| [Crossterm](https://github.com/crossterm-rs/crossterm) | Terminal manipulation + input     |
| [Rayon](https://github.com/rayon-rs/rayon)             | Parallel iteration                |
| [WalkDir](https://github.com/BurntSushi/walkdir)       | Recursive file system traversal   |
| [Calamine](https://github.com/tafia/calamine)          | Excel reader                      |
| [lopdf](https://github.com/J-F-Liu/lopdf)              | PDF reader                        |
| [zip](https://github.com/zip-rs/zip2)                  | ZIP/DOCX reader                   |
| [cfb](https://github.com/mdsteele/rust-cfb)            | Compound File Binary (DOC) reader |
| [memchr](https://github.com/BurntSushi/memchr)         | Fast byte-pattern search          |

## License

MIT — see [LICENSE](LICENSE).
