# Keybindings

Canopy is modal (vim-style). The active context is shown by the colored block at
the left of the status bar (`NORMAL`, `INSERT`, `COMMAND`, `BROWSE`, `PDF`, `AI`).

## Global

| Key | Action |
| --- | --- |
| `Ctrl-C` | Quit immediately (from any context) |

## File browser (`BROWSE`)

Shown when Canopy is launched on a directory (or after `:e` / `Ctrl-O`).

| Key | Action |
| --- | --- |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Jump to first entry |
| `G` | Jump to last entry |
| `Enter` / `l` | Open file / enter directory |
| `h` / `←` / `Backspace` | Go to parent directory |
| `/` or `Ctrl-F` | Open the fuzzy file finder |
| `Ctrl-A` | Open the AI panel (if a document is open) |
| `q` / `Ctrl-Q` | Quit |

## Fuzzy file finder (`Ctrl-F`)

A centered overlay; scoped to the enclosing git repo.

| Key | Action |
| --- | --- |
| _type_ | Filter the file list |
| `↓` / `Ctrl-N` | Next match |
| `↑` / `Ctrl-P` | Previous match |
| `Backspace` | Delete a character from the query |
| `Enter` | Open the selected file |
| `Esc` | Close the finder |

## Workspace

The editor screen has three panes: **editor**, **PDF preview**, and **AI**.

| Key | Action |
| --- | --- |
| `Ctrl-W` | Cycle focus between visible panes (editor → PDF → AI) |
| `Ctrl-A` | Focus the AI panel |
| `Ctrl-P` | Toggle the PDF preview pane |
| `Ctrl-O` | Open the file browser |
| `Ctrl-F` | Open the fuzzy finder |
| `Ctrl-B` | Compile (sandboxed Docker) |
| `Ctrl-S` | Save |
| `Ctrl-Q` | Quit |

### Editor — Normal mode (`NORMAL`)

| Key | Action |
| --- | --- |
| `h` `j` `k` `l` / arrows | Move left / down / up / right |
| `w` | Next word |
| `b` | Previous word |
| `0` / `Home` | Start of line |
| `$` / `End` | End of line |
| `gg` | First line |
| `G` | Last line |
| `Ctrl-D` / `Ctrl-U` | Half-page down / up |
| `PageDown` / `PageUp` | Page down / up |
| `i` | Insert before cursor |
| `a` | Insert after cursor |
| `I` | Insert at start of line |
| `A` | Insert at end of line |
| `o` / `O` | Open a new line below / above |
| `x` | Delete character under cursor |
| `dd` | Delete the current line |
| `D` | Delete to end of line |
| `:` | Enter the command line |

### Editor — Insert mode (`INSERT`)

| Key | Action |
| --- | --- |
| _type_ | Insert text |
| `Esc` | Return to Normal mode |
| arrows / `Home` / `End` / `PageUp` / `PageDown` | Move the cursor |
| `Enter` | New line |
| `Tab` | Insert two spaces |
| `Backspace` | Delete the character before the cursor |
| `Delete` | Delete the character under the cursor |

### Editor — Command line (`:`)

| Command | Action |
| --- | --- |
| `:w` | Write (save) |
| `:q` | Quit (warns on unsaved changes) |
| `:q!` | Quit, discarding changes |
| `:wq` / `:x` | Write and quit |
| `:e` | Open the file browser |
| `:make` | Compile |
| `:pdf` | Toggle the PDF preview pane |
| `:ai` | Toggle the AI panel |
| `Esc` | Cancel the command line |

### PDF preview (`PDF`) — focus with `Ctrl-W`

| Key | Action |
| --- | --- |
| `j` `k` `h` `l` / arrows | Scroll down / up / left / right |
| `+` / `-` | Zoom in / out |
| `0` | Reset zoom and scroll |
| `n` / `PageDown` | Next page |
| `p` / `PageUp` | Previous page |
| `g` / `G` | First / last page |
| `Esc` | Return focus to the editor |

### AI assistant (`AI`) — focus with `Ctrl-A`

Backed by a local Ollama model (default `gemma4:12b-it-qat`).

| Key | Action |
| --- | --- |
| _type_ | Compose a message |
| `Enter` | Send (streams the reply; the current document is included as context) |
| `PageUp` / `PageDown` | Scroll the conversation |
| `Backspace` | Delete a character |
| `Esc` | Stop a streaming reply, or return focus to the editor |
| `Ctrl-A` | Return focus to the editor |
