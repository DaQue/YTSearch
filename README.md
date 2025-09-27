# YTSearch

A tiny desktop tool (Rust + egui) to search YouTube with **strict filters**:
- Date window (Today / 48h / 7d / Custom)
- English-leaning results
- Avoid Shorts (duration >= threshold)
- Subject-limited search (terms, channel allow/deny, category)
- Saved **My Searches** presets with Single / Any run modes

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
- Hit **Search** (currently logs; fill in the HTTP calls in `src/yt/*.rs`).

See `CONTRIBUTING.md` for development details and `CHANGELOG.md` for release notes.

## Current Status (2025-09-27)

✅ **WORKING** - Multi-key fallback system operational!
- Successfully tested with 3 API keys and sequential fallback
- Comprehensive error handling with detailed YouTube API diagnostics
- CLI probe confirmed: 2 presets, 5 pages, 112 videos retrieved in last 24h test

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
