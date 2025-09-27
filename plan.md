# Implementation Plan

1. **Stabilize preference and query models**
   - Align the structs in `src/prefs.rs` with the preset schema described in `YTSearch_Plan.md`.
   - Add defaults and helpers for resolving overrides.
   - Implement JSON import/export utilities so UI dialogs can round-trip `MySearch` payloads.

2. **Flesh out YouTube client plumbing**
   - Extend `src/yt/search.rs` and `src/yt/videos.rs` with parameter builders for date windows, pagination, caption, and region filters.
   - Add conversion helpers so API responses map cleanly into `VideoDetails` in `src/yt/types.rs`.

3. **Implement filter and merge logic**
   - Add `matches_post_filters` and related utilities in `src/filters.rs` to enforce duration, language, channel allow/deny, and term rules.
   - Provide functions to merge multi-preset results (Single vs Any modes) with dedupe and sort guarantees.

4. **Build the async search runner**
   - Introduce a controller (e.g., `search_runner.rs`) that spawns Tokio tasks from the UI.
   - Orchestrate `search.list` followed by `videos.list`, handle paging/quota budgeting, update app state, and report errors/status back to `AppState` in `src/ui.rs`.

5. **Flesh out egui UX**
   - Replace the placeholders in the UI with preset management dialogs (new/edit/duplicate/import/export) per `YTSearch_Plan.md`.
   - Add Any/Single selectors, API-key entry, progress indicators, and a results list showing thumbnails, metadata, and clickable links via the `open` crate.

6. **Polish, document, and validate**
   - Surface non-blocking errors in the status bar, add integration/unit tests for prefs and filters.
   - Update `README.md` with setup/run notes and packaging steps.
   - Sanity-check builds on targeted platforms.
