# YTSearch (starter repo)

A tiny desktop tool (Rust + egui) to search YouTube with **strict filters**:
- Date window (Today / 48h / 7d / Custom)
- English-leaning results
- Avoid Shorts (duration >= threshold)
- Subject-limited search (terms, channel allow/deny, category)
- Saved **My Searches** presets with Single / Any run modes

This is a **compile-ready skeleton** with TODOs where Codex (or you) can fill in logic.
It includes stubs for `search.list` and `videos.list` calls and a simple UI shell.

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

See `YTSearch_Plan.md` for a deeper plan.
