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

## Outstanding / Follow-up Ideas
- Implement actual preset editor (New / Edit / Duplicate) instead of placeholder message.
- Provide Reset-to-defaults button (restore defaults, clear block list).
- Consider optional higher `maxResults` (25 → 50) and/or bump `MAX_SEARCH_PAGES` strategy to reduce API calls.
- Improve Single vs Any affordance further (maybe icon/toggle badge).
- Provide channel-unblock confirmation / history.
- Explore saved block list separate from prefs.json (e.g., dedicated file).
