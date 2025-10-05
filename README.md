# YTSearch

A tiny desktop tool (Rust + egui) to search YouTube with **strict filters**:
- Date window (Today / 48h / 7d / Any date)
- English-leaning results
- Avoid Shorts (duration >= threshold or multi-select length buckets)
- Subject-limited search (terms, channel allow/deny, category)
- Saved **My Searches** presets with Single / Any run modes
- Cached results reopen instantly without spending quota
- Result sorting (Newest, Oldest, Shortest, Longest, Channel)
- In-app Help dialog (top-right) summarises version info and API key setup steps

✅ **Fully functional** YouTube search tool with robust API handling and modern UI.

## Build

```bash
cargo build
cargo run
```

If egui versions mismatch in your toolchain, keep eframe+egui versions **in sync**.

## Where to start

- Put your Google **API Key** in the app's Settings panel (stored in `prefs.json` under your OS config dir).
- Open the left panel and create a few **My Searches** presets.
- Use the built-in preset editor (New / Edit / Duplicate / Import / Export) to tune subject filters without touching JSON. Copy/paste single presets from the clipboard or bulk load/export JSON as needed.
- Hit **Search** (currently logs; fill in the HTTP calls in `src/yt/*.rs`).

See `CONTRIBUTING.md` for development details and `CHANGELOG.md` for release notes.

## Current Status (2025-09-27)

🚧 **FUNCTIONAL** - Core search working, UI continues to improve
- Multi-key fallback system operational (3 API keys with sequential retry)
- Cached results reload on launch (banner auto-hides after 5s to keep controls visible)
- Duration filters support mix-and-match buckets with automatic "Any" fallback
- Result list can be sorted by newest/oldest, shortest/longest, or channel
- Preset editor supports clipboard copy/paste, replace-vs-append imports, and scrollable forms for long configurations
- Known issues: API key UI missing, preset editor still uses chip-style manual term entry

### Setup / Usage

- Place your API keys in these files for automatic fallback on 403 errors:
  - `YT_API_private` - Primary key (loaded into UI if prefs.json is empty)
  - `YT_API_private.alt` - First fallback key  
  - `YT_API_private,old` - Second fallback key
  The client will try each key in sequence when quota/key issues occur.
- Conserve quota by limiting pages per run via environment variable:
  ```bash
  YTSEARCH_MAX_SEARCH_PAGES=1 cargo run --bin probe -- --hours 24 --limit 5
  ```
- In Google Cloud for your key:
  - Ensure YouTube Data API v3 is enabled
  - Remove HTTP referrer restrictions (desktop apps). Use None or IP restrictions that match your machine
  - If API restrictions are set, include `youtube.googleapis.com`
