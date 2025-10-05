# Changelog

All notable changes to YTSearch will be documented in this file.

# [Unreleased]

### ✨ Added
- Clipboard copy/paste support for individual presets, including dirty-state confirmation before replacing unsaved edits.
- Import dialog option to replace existing presets or append to the current list, with scrollable JSON editors for large payloads.
- Scrollable preset editor layout so Save/Cancel controls remain accessible even with extensive configuration.
- Cached search results persist between launches and reload automatically without hitting the API.
- Result sorting controls (Newest, Oldest, Shortest, Longest, Channel) in the results header.

### 🛠️ Changed
- Status and button labels now match the preset workflow ("Load presets" / "Save presets").
- Duration bucket chips now allow multi-select and automatically fall back to "Any length" when cleared.
- Cached banner copy shortened and set to auto-hide after 5 seconds so the search button remains visible at narrow widths.

## [0.1.0] - 2025-09-27

### 🎉 Initial Release

#### ✨ Features
- **Multi-preset YouTube search** with strict filtering
  - Date windows: Today, 48h, 7d, Any date
  - English-first results (audio language + captions + title heuristics)
  - Duration filtering to avoid YouTube Shorts (configurable threshold)
  - Subject-limited search with terms, channel allow/deny lists
- **Run modes**: Single preset or Any (union of all enabled presets)
- **Channel blocking**: Persistent block list with UI management
- **Modern UI**: Dark theme with rainbow accent colors, responsive design
- **CLI probe tool**: Command-line testing and parameter inspection

#### 🔧 Technical
- **Multi-key API fallback**: Sequential retry through 3 API key files
- **Robust error handling**: Detailed YouTube API diagnostics with specific error reasons
- **Async architecture**: Tokio runtime with non-blocking UI updates
- **Modular codebase**: Clean separation of concerns (UI, search, filters, prefs)
- **Cross-platform**: Windows & Linux support

#### 📁 Key Files
- `YT_API_private` - Primary API key (auto-loaded if prefs empty)
- `YT_API_private.alt` - First fallback key  
- `YT_API_private,old` - Second fallback key
- `~/.config/YTSearch/prefs.json` - Saved preferences and presets

#### 🚀 Usage
```bash
# Desktop UI
cargo run

# CLI testing
cargo run --bin probe -- --hours 24 --limit 10

# Quota conservation
YTSEARCH_MAX_SEARCH_PAGES=1 cargo run --bin probe -- --dry-run
```

### 🐛 Known Issues
- **Preset editor**: UI shows "not implemented" placeholder (no New/Edit/Delete operations)
- **Channel names**: Displays channel IDs (e.g. "UC1234...") instead of readable names (e.g. "Rust", "Colin")
- **API key management**: No UI to change key in settings (must edit YT_API_private files manually)
- **UI polish**: Missing thumbnails, limited keyboard shortcuts, basic layout
- **OAuth**: No captions.list API support yet (precise English caption detection planned)

### 🙏 Acknowledgments  
- Built with egui, tokio, reqwest, and serde
- YouTube Data API v3 integration
- Inspired by the need for **actual** YouTube search filters
