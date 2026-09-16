# refrain

### A fast, lightweight and modern terminal music for Linux.

Built with Rust and Ratatui.

<br />

<img src="./.github/assets/screenshot.png" alt="Refrain Screenshot" width="800" />

<br />

> **Status: Active Work In Progress**
> refrain is currently in early development. Features, keybindings and configuration may change.

## Features (marked have been finished)
- [x] Cover art display - displays cover art of all songs in the list
- [x] Audio waveform - shows a seekbar in the form of an audio waveform, powered by [`audio-waveform`](https://github.com/s1nn3rv2/audio-waveform)
- [x] Fuzzy search - fuzzy-search powered by [`nucleo-matcher`](https://github.com/helix-editor/nucleo) 
- [x] Extremely fast - scanning 1800+ songs takes less than a second, covers load on demand
- [x] Tag editing - allows you to edit tags on songs
- [x] MPRIS support - allows you to control and see the current song in your system's media player 
- [x] Queue management - add tracks, play next, view and manage via collapsible drawer
- [ ] Context-aware shuffle - knows your context, whether you're in an album, genre or everything, it knows what songs to pick from
- [ ] Synced lyrics - automatically fetches and scrolls lyrics 
- [ ] Gapless playback - seamless track transitions
- [ ] Extensible config support - allows you to adjust the player to your liking 

## Keybindings

### Navigation & Playback

| Key | Action |
|:---|:---|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `Tab` / `Shift + Tab` | Cycle focus between panes (Sidebar, Library, Queue) |
| `Enter` / `l` | Play selected track |
| `p` | Play / Pause |
| `n` / `>` | Next track |
| `u` | Toggle queue drawer open / closed |
| `s` | Cycle sort column |
| `S` | Toggle sort direction (ascending/descending) |
| `/` | Open fuzzy search |
| `r` | Rescan `~/Music` directory |
| `Esc` / `Enter` | Exit search mode |
| `q` | Quit |

### Queue Actions

| Key | Context | Action |
|:---|:---|:---|
| `a` | Library | Add selected track to queue |
| `A` | Library | Play selected track next (top of queue) |
| `Enter` / `l` | Queue | Play selected queued track |
| `d` | Queue | Remove selected track from queue |
| `K` / `Shift + Up` | Queue | Move selected track up |
| `J` / `Shift + Down` | Queue | Move selected track down |

### Sidebar

| Key | Context | Action |
|:---|:---|:---|
| `[` / `Left` | Sidebar | Previous category tab (Genres, Albums, Artists) |
| `]` / `Right` | Sidebar | Next category tab (Genres, Albums, Artists) |
| `j` / `Down` | Sidebar | Move selection down |
| `k` / `Up` | Sidebar | Move selection up |
| `Enter` | Sidebar | Apply selected filter (or clear if "All" is selected) |

### Tag Editor

| Key | Context | Action |
|:---|:---|:---|
| `e` | Library | Open tag editor for selected track |
| `Tab` / `Enter` | Tag Editor | Move to next field |
| `Shift + Tab` | Tag Editor | Move to previous field |
| `Ctrl + s` | Tag Editor | Save changes to file & update library |
| `Esc` | Tag Editor | Cancel and close modal |

## Configuration

Refrain looks for a configuration file at `~/.config/refrain/config.toml`. If the file does not exist, default settings are used.

```toml
# Path to your music library (defaults to ~/Music)
music_dir = "~/Music"

# Simple column setup (uses default widths and alignments)
columns = [
    "cover",
    "artist",
    "title",
    "album",
    "length"
]

# Fully custom columns:
# columns = [
#    "cover"
#    { name = "artist", label = "Artist", width = "25%" },
#    { name = "title", width = "fill"},
#    { name = "album", label = "Album", width = "20%" },
#    { name = "genre", labbel = "Genre", width = "15%" },
#    { name = "date", label = "Year", width = "4", alignment = "center" },
#    { name = "length", label = "Time", width = "8", alignment = "right"}
#]
```

Available columns:
| Name | Description | Default Width | Default Alignment |
|:---|:---|:---|:---|
|cover|Cover art thumbnail|7 chars|Left|
|title|Track title|fill|Left|
|artist|Track artists|20|Left|
|album|Album name|20%|Left|
|genre|Genre|15%|Left|
|date|Release date/year|10 chars|Left|
|length|Track duration|8 chars|Right|

---

## Why not rmpc?
Rmpc is a really good option as well! But I missed having cover arts displayed on every song. At first I thought it was just a limitation of TUI apps, but after seeing [concord](https://github.com/chojs23/concord) I realized it's possible! I also wanted a workflow that I had when I was using foobar2000 back on Windows, where I also could edit tags of songs directly from the app. Despite me making this app mainly for myself, I do want to add a lot of customization options for it to be adjustable by the user as well!

## Uses
- **TUI:** [Ratatui](https://github.com/ratatui/ratatui) & [Crossterm](https://github.com/crossterm-rs/crossterm)
- **Audio playback:** [Rodio](https://github.com/RustAudio/rodio)
- **Tag extraction:** [Lofty](https://github.com/Serial-ATA/lofty-rs)
- **Waveforms:** [audio-waveform](https://github.com/s1nn3rv2/audio-waveform) 
- **Image support:** [ratatui-image](https://github.com/benjajaja/ratatui-image)
- **Fuzzy search:** [nucleo-matcher](https://github.com/helix-editor/nucleo)
- **MPRIS D-Bus integration:** [mpris-server](https://github.com/SeaDve/mpris-server)

## Build & Run

```bash
# Clone the repository
git clone https://github.com/s1nn3rv2/refrain.git
cd refrain

# Run in release mode (it is WAY faster in release mode)
cargo run --release
```

## Contributing

Contributions, bug reports and feature ideas are very welcome!

## License

Licensed under the MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT). 
