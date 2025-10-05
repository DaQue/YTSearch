# YTSearch - Current Status Snapshot (2025-09-27)

## What’s Done
- UI restyle mirrors gfv palette: rainbow top controls, dark cards, default window size 1100×720 with 1100 min width.
- Dual presets ship by default (Rust Programming + Sovereign Citizen Watch); Any mode runs both.
- Block channel button removes videos immediately and persists key|label pairs in prefs; blocked list shown in sidebar with Unblock buttons.
- Async search keeps spinner text (“Searching…”), updates status bar with raw/unique/passed/kept counts once finished.
- “Open” button prefers `google-chrome --new-window`, `chromium --new-window`, `brave-browser --new-window`, or `microsoft-edge --new-window`; falls back to default handler.
- CLI probe (`cargo run --bin probe`) respects the same defaults/blocked list.
- Build clean: `cargo fmt`, `cargo check`, `cargo clippy`, `cargo build --release`.
- Repo committed (`UI restyle with multi-preset defaults and channel blocking`).
- Preset editor UI supports New/Edit/Duplicate, plus JSON import/export workflows.

### Networking / API
- Added explicit YouTube error parsing and nicer messages (endpoint + reason).
- Auto-fallback through multiple API keys (`YT_API_private.alt`, `YT_API_private,old`, `YT_API_private`) when 403 appears quota/key related.
- Env override to cap pages per run: `YTSEARCH_MAX_SEARCH_PAGES` (default 2).

## Outstanding / Follow-up Ideas
- Provide Reset-to-defaults button (restore defaults, clear block list).
- Improve Single vs Any affordance further (maybe icon/toggle badge).
- Provide channel-unblock confirmation / history.
- Explore saved block list separate from prefs.json (e.g., dedicated file).
- Make duration buckets (labels + ranges) editable from UI.

## Current Status: 🚧 FUNCTIONAL BUT INCOMPLETE

### ✅ What's Working
- **Multi-key fallback operational**: Successfully tested with 3 API keys
- **API calls working**: Probe retrieved 112 videos across 5 pages in 24h test
- **Error handling robust**: Detailed YouTube API diagnostics and sequential key retry
- **UI refactor complete**: Clean modular structure (663 lines → 5 focused modules)

### 🚧 Known Issues
- **API key UI**: No way to change key in settings panel (must edit files)
- **Missing features**: Limited keyboard shortcuts, further preset editor polish needed
- **Preset editor polish**: Add drag-and-drop reordering and richer validation messaging for term chips
