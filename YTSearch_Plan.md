
# YTSearch — Week-Sized Vibe Project (Rust + egui)

A tiny desktop tool that searches YouTube with **actual hard filters**:
- Strict **date window** (Today / 48h / 7d / Custom).
- **English-first** results (audio language and/or English captions).
- **No Shorts** (via duration threshold).
- **Subject‑limited** search (topic terms, channel allow/deny, category).
- Saved **My Searches** presets you can run **individually** or as an **Any** union feed.

This file is everything you need to hand to Codex (or paste into a new repo) to scaffold the app quickly, with fewer back‑and‑forths.

---

## 0) Quick Start (hand this to Codex)

**Task:** “Create a Rust eframe/egui desktop app named `YTSearch` that implements the structure and types below. Add YouTube API client calls for `search.list` and `videos.list`. Render a basic UI with a left panel of *My Searches* and a results list. Provide a `README.md` with setup steps.”

**Steps Codex should follow:**
1. Generate `Cargo.toml` (see **Cargo.toml** template below).
2. Scaffold the `src/` layout shown under **Project Layout**.
3. Implement `yt::search::search_list()` and `yt::videos::videos_list()` using `reqwest + serde`.
4. Wire a minimal egui UI: input API key, choose preset (Today/48h/7d), select **Single** vs **Any** run, list results (thumbnail, title, channel, published, duration), click opens in browser.
5. Implement `filters.rs` rules (date, duration >= 75s as default, English heuristic with `defaultAudioLanguage` / `defaultLanguage`).
6. Load/save `prefs.json` to store API key and *My Searches*.
7. Add **My Searches** editor with Enable/Disable toggles and Import/Export JSON.
8. Build release binaries for Windows & Linux, and fill the README’s setup instructions.

> **Note:** Optional OAuth for `captions.list` (precise English caption detection) is a **stretch goal**; not required for v1.

---

## 1) Architecture Overview

- **Runtime:** Rust + Tokio
- **GUI:** `egui` + `eframe`
- **HTTP/JSON:** `reqwest`, `serde`, `serde_json`
- **Config:** JSON in per‑OS app dir via `directories`
- **Open link:** `open`
- **(Optional later)** OAuth + `captions.list`

### Data Flow
1. `search.list`: pass `q`, `type=video`, `publishedAfter/Before`, `order=date`, `videoCaption=closedCaption` (optional), `videoDuration` (optional), `pageToken`.
2. Collect `videoId`s → `videos.list` (`part=snippet,contentDetails`) to get `publishedAt`, `duration`, `defaultAudioLanguage`/`defaultLanguage`, thumbnails.
3. Client filters: date window, duration >= threshold (avoid Shorts), English heuristic, not‑terms/channel allow/deny.
4. Sort strictly by `publishedAt` desc. De‑dupe across presets in **Any** mode.

---

## 2) My Searches (Presets)

### Data Model (serialize to `prefs.json`)
```rust
// src/prefs.rs
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
pub struct Prefs {
    pub api_key: String,
    pub global: GlobalPrefs,
    pub searches: Vec<MySearch>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct GlobalPrefs {
    pub default_window: TimeWindowPreset, // Today|H48|D7|Custom
    pub english_only: bool,
    pub require_captions: bool,
    pub verify_captions_with_oauth: bool,
    pub min_duration_secs: u32,           // e.g., 75 (avoid Shorts)
    pub region_code: Option<String>,      // e.g., "US"
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MySearch {
    pub id: String,                       // UUID/slug
    pub name: String,                     // "Electronics Repair"
    pub enabled: bool,
    pub query: QuerySpec,
    pub window_override: Option<TimeWindow>,
    pub english_only_override: Option<bool>,
    pub require_captions_override: Option<bool>,
    pub min_duration_override: Option<u32>,
    pub priority: i32,                    // order in Any mode (higher first)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct QuerySpec {
    pub q: Option<String>,                // free text
    pub any_terms: Vec<String>,           // OR
    pub all_terms: Vec<String>,           // AND
    pub not_terms: Vec<String>,           // NOT
    pub channel_allow: Vec<String>,       // channel IDs or @handles
    pub channel_deny: Vec<String>,
    pub category_id: Option<u32>,         // e.g., 28 (Sci & Tech)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub enum TimeWindowPreset { Today, H48, D7, Custom }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TimeWindow {
    pub start_rfc3339: String,
    pub end_rfc3339: String,
}
```

### Example `prefs.json`
```json
{
  "api_key": "YOUR_KEY",
  "global": {
    "default_window": "D7",
    "english_only": true,
    "require_captions": false,
    "verify_captions_with_oauth": false,
    "min_duration_secs": 75,
    "region_code": "US"
  },
  "searches": [
    {
      "id": "elec-repair",
      "name": "Electronics Repair",
      "enabled": true,
      "query": {
        "q": null,
        "any_terms": ["electronics repair","board repair","reball","recap"],
        "all_terms": [],
        "not_terms": ["shorts","#shorts","asmr"],
        "channel_allow": ["@LouisRossmann","@shango066"],
        "channel_deny": [],
        "category_id": 28
      },
      "window_override": null,
      "english_only_override": null,
      "require_captions_override": null,
      "min_duration_override": 180,
      "priority": 10
    },
    {
      "id": "rust-dev",
      "name": "Rust Dev",
      "enabled": true,
      "query": {
        "q": "rust programming",
        "any_terms": ["egui","iced","slint","wasm","embedded"],
        "all_terms": [],
        "not_terms": ["shorts","podcast"],
        "channel_allow": [],
        "channel_deny": [],
        "category_id": 28
      },
      "window_override": { "start_rfc3339": "2025-09-19T00:00:00Z", "end_rfc3339": "2025-09-26T23:59:59Z" },
      "english_only_override": true,
      "require_captions_override": false,
      "min_duration_override": 120,
      "priority": 5
    }
  ]
}
```

---

## 3) UI Sketch (egui)

**Left Panel — My Searches**
- Filter box (fuzzy search your presets)
- List: `[✓]` enable, drag‑handle, name, ⋮ menu (Edit / Duplicate / Export / Delete)
- Buttons: **New**, **Import JSON**, **Export All**

**Top Bar — Run Controls**
- Run mode: **Single | Any**
- If Single → dropdown of presets
- Window presets: **Today | 48h | 7d | Custom**
- Toggles: **English only**, **Require captions**, **No Shorts (<75s)**
- **Search** button

**Main — Results**
- Row: thumbnail • title • channel • published time • duration • matched‑preset tags
- Click → open in browser
- Footer: “N results · page X/Y · ~quota units”

Keyboard: ↑/↓ to move in presets, **Enter** to run, **Space** enable/disable, **E** edit.

---

## 4) Implementation Order (1–3 hr chunks)

**Day 1 — Skeleton + Key**
- eframe bootstrap; config (`directories`) for `prefs.json` (API key + toggles)
- Implement `yt::search::search_list()` (log IDs + `nextPageToken`)

**Day 2 — Details + Filters**
- `yt::videos::videos_list()` (`part=snippet,contentDetails`)
- ISO‑8601 duration → seconds
- Filters: date window, duration >= threshold, English heuristic

**Day 3 — Render**
- egui results list with thumbnails & open‑in‑browser
- Show quota estimate; smooth scrolling

**Day 4 — Presets & Paging**
- Presets: Today/48h/7d
- “Load more” → `pageToken`
- “Require captions” → add `videoCaption=closedCaption` in search call
- Persist last query/toggles

**Day 5 — My Searches**
- Left panel CRUD (enable/disable, edit, duplicate)
- Import/Export JSON
- **Any** mode: run all enabled searches, de‑dupe by `videoId`, sort by `publishedAt` desc with tags

**Day 6 — Hardening**
- Error UI for HTTP/YouTube errors; retries with jitter
- Small in‑memory cache of `videos.list` for a page

**Day 7 — Polish & Build**
- Exclude keywords; regionCode
- Windows/Linux release builds
- README with setup + screenshots

---

## 5) Query Builder Rules

### Compose `q`
```
q_string = [q]
         + (any_terms.is_empty ? [] : ["(" + join(" OR ", any_terms) + ")"])
         + all_terms
         + not_terms.map(t => "-"+t)
joined with spaces
```

### `search.list` Params (minimum)
- `part=snippet`
- `type=video`
- `q=q_string` (subject filtering)
- `order=date`
- `maxResults=25` (tune 25–50)
- `publishedAfter` and `publishedBefore` (RFC3339)
- Optional: `videoCaption=closedCaption`, `videoCategoryId`, `regionCode`, `videoDuration=medium|long`
- `pageToken` for paging

### `videos.list` Params
- `part=snippet,contentDetails`
- `id=<comma_separated_video_ids>` (<= 50 per call)

---

## 6) Post‑Filters (exact)

```rust
fn matches_post_filters(v: &VideoDetails, prefs: &GlobalPrefs, s: &MySearch) -> bool {
    // Duration
    let min_secs = s.min_duration_override.unwrap_or(prefs.min_duration_secs);
    if v.duration_secs < min_secs { return false; }

    // English
    let want_en = s.english_only_override.unwrap_or(prefs.english_only);
    if want_en {
        let lang_ok = v.default_audio_lang.as_deref().unwrap_or("").starts_with("en")
            || v.default_lang.as_deref().unwrap_or("").starts_with("en")
            || v.has_caption_lang_en.unwrap_or(false); // set if captions verified later
        if !lang_ok { return false; }
    }

    // NOT terms (title only for v1; add description later)
    if contains_any(&v.title_lower, &s.query.not_terms) { return false; }

    // Channel allow/deny (match by handle if available)
    if !s.query.channel_deny.is_empty() && matches_channel(&v.channel_handle, &s.query.channel_deny) {
        return false;
    }
    if !s.query.channel_allow.is_empty() && !matches_channel(&v.channel_handle, &s.query.channel_allow) {
        return false;
    }
    true
}
```

---

## 7) Error Handling Playbook

- Show which endpoint failed and hint (invalid key, quota, missing fields).
- **quotaExceeded**: disable caption verification (if you add OAuth later), suggest smaller pages.
- Network: retry 250ms → 500ms → 1s with jitter.
- Graceful empty states (no results, no internet).

---

## 8) Stretch Goals

- Export results to **HTML** poster or **CSV**.
- Saved searches panel with quick hotkeys & auto‑refresh (off by default).
- Channel Uploads playlist route for favorite channels.
- OAuth + `captions.list` for exact English caption filtering.

---

## 9) Project Layout

```
YTSearch/
  Cargo.toml
  README.md
  src/
    main.rs
    ui.rs
    prefs.rs
    filters.rs
    yt/
      mod.rs
      search.rs
      videos.rs
      types.rs
```

---

## 10) Cargo.toml (template)

```toml
[package]
name = "YTSearch"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.31"
egui = "0.31"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time"] }
reqwest = { version = "0.12", features = ["json", "gzip", "brotli", "deflate", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_repr = "0.1"
thiserror = "1"
urlencoding = "2"
directories = "5"
open = "5"
time = { version = "0.3", features = ["parsing", "macros"] }
```

> If your current egui/eframe version is different, keep them in sync (you’ve used 0.31 before).

---

## 11) Minimal Code Stubs

### `src/main.rs`
```rust
mod ui;
mod prefs;
mod filters;
mod yt;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "YTSearch",
        native_options,
        Box::new(|cc| Box::new(ui::AppState::new(cc))),
    )
}
```

### `src/ui.rs`
```rust
use crate::prefs::{Prefs, GlobalPrefs, TimeWindowPreset};
use crate::yt::{search::search_list, videos::videos_list};
use crate::filters;
use egui::{Context};

pub struct AppState {
    prefs: Prefs,
    status: String,
    results: Vec<crate::yt::types::VideoDetails>,
}

impl AppState {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let prefs = crate::prefs::load_or_default();
        Self { prefs, status: String::new(), results: Vec::new() }
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.label("YTSearch");
            // TODO: add controls: API key field, window preset, Single/Any, Search button
        });
        egui::SidePanel::left("left").show(ctx, |ui| {
            ui.heading("My Searches");
            // TODO: list + enable/disable + edit
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&self.status);
            // TODO: results list with thumbnails
        });
    }
}
```

### `src/prefs.rs`
```rust
use directories::ProjectDirs;
use serde::{Serialize, Deserialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Prefs {
    pub api_key: String,
    pub global: GlobalPrefs,
    pub searches: Vec<MySearch>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GlobalPrefs {
    pub default_window: TimeWindowPreset,
    pub english_only: bool,
    pub require_captions: bool,
    pub verify_captions_with_oauth: bool,
    pub min_duration_secs: u32,
    pub region_code: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct MySearch {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub query: QuerySpec,
    pub window_override: Option<TimeWindow>,
    pub english_only_override: Option<bool>,
    pub require_captions_override: Option<bool>,
    pub min_duration_override: Option<u32>,
    pub priority: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QuerySpec {
    pub q: Option<String>,
    pub any_terms: Vec<String>,
    pub all_terms: Vec<String>,
    pub not_terms: Vec<String>,
    pub channel_allow: Vec<String>,
    pub channel_deny: Vec<String>,
    pub category_id: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum TimeWindowPreset { Today, H48, D7, Custom }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeWindow { pub start_rfc3339: String, pub end_rfc3339: String }

pub fn load_or_default() -> Prefs {
    let path = prefs_path();
    if let Ok(bytes) = fs::read(&path) {
        if let Ok(p) = serde_json::from_slice::<Prefs>(&bytes) { return p; }
    }
    Prefs {
        api_key: String::new(),
        global: GlobalPrefs {
            default_window: TimeWindowPreset::D7,
            english_only: true,
            require_captions: false,
            verify_captions_with_oauth: false,
            min_duration_secs: 75,
            region_code: Some("US".into()),
        },
        searches: vec![],
    }
}

pub fn save(p: &Prefs) -> std::io::Result<()> {
    let path = prefs_path();
    if let Some(dir) = path.parent() { fs::create_dir_all(dir)?; }
    fs::write(path, serde_json::to_vec_pretty(p)?)
}

fn prefs_path() -> PathBuf {
    let proj = ProjectDirs::from("com", "yourname", "YTSearch")
        .expect("no project dirs");
    proj.config_dir().join("prefs.json")
}
```

### `src/filters.rs`
```rust
use crate::prefs::{GlobalPrefs, MySearch};

pub fn parse_iso8601_duration(s: &str) -> Option<u64> {
    // extremely simple parser for PT#H#M#S; expand as needed
    let (mut h, mut m, mut sec) = (0u64, 0u64, 0u64);
    if !s.starts_with('P') { return None; }
    let t = s.split('T').nth(1)?;
    let mut num = String::new();
    for ch in t.chars() {
        if ch.is_ascii_digit() { num.push(ch); continue; }
        let val: u64 = num.parse().ok()?; num.clear();
        match ch {
            'H' => h = val,
            'M' => m = val,
            'S' => sec = val,
            _ => {}
        }
    }
    Some(h*3600 + m*60 + sec)
}

pub fn contains_any(hay: &str, needles: &[String]) -> bool {
    let h = hay.to_ascii_lowercase();
    needles.iter().any(|n| h.contains(&n.to_ascii_lowercase()))
}
```

### `src/yt/mod.rs`
```rust
pub mod search;
pub mod videos;
pub mod types;
```

### `src/yt/types.rs`
```rust
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct VideoDetails {
    pub id: String,
    pub title: String,
    pub title_lower: String,
    pub channel_title: String,
    pub channel_handle: String,
    pub published_at: String,
    pub duration_secs: u64,
    pub default_audio_lang: Option<String>,
    pub default_lang: Option<String>,
    pub thumbnail_url: Option<String>,
    pub url: String,
    pub has_caption_lang_en: Option<bool>, // set later if you add captions.list
}

// Minimal response structs (fill as needed)
#[derive(Deserialize)]
pub struct SearchListResponse {
    pub nextPageToken: Option<String>,
    pub items: Vec<SearchItem>,
}
#[derive(Deserialize)]
pub struct SearchItem {
    pub id: SearchId,
    pub snippet: Snippet,
}
#[derive(Deserialize)]
pub struct SearchId { pub videoId: Option<String> }
#[derive(Deserialize)]
pub struct Snippet { pub publishedAt: String }

#[derive(Deserialize)]
pub struct VideosListResponse { pub items: Vec<VideoItem> }
#[derive(Deserialize)]
pub struct VideoItem {
    pub id: String,
    pub snippet: VideoSnippet,
    pub contentDetails: ContentDetails,
}
#[derive(Deserialize)]
pub struct VideoSnippet {
    pub title: String,
    pub channelTitle: String,
    pub channelId: String,
    pub publishedAt: String,
    pub defaultAudioLanguage: Option<String>,
    pub defaultLanguage: Option<String>,
    pub thumbnails: Option<Thumbs>,
}
#[derive(Deserialize)]
pub struct Thumbs { #[serde(rename="medium")] pub medium_: Option<Thumb> }
#[derive(Deserialize)]
pub struct Thumb { pub url: String }
#[derive(Deserialize)]
pub struct ContentDetails { pub duration: String }
```

### `src/yt/search.rs`
```rust
use super::types::SearchListResponse;

pub async fn search_list(
    api_key: &str,
    params: &[( &str, String )],
) -> anyhow::Result<SearchListResponse> {
    let mut url = "https://www.googleapis.com/youtube/v3/search".to_string();
    url.push_str("?part=snippet&type=video");
    for (k,v) in params {
        url.push('&'); url.push_str(k); url.push('='); url.push_str(&urlencoding::encode(v));
    }
    url.push_str("&key="); url.push_str(api_key);

    let resp = reqwest::Client::new()
        .get(&url).send().await?
        .error_for_status()?;
    Ok(resp.json::<SearchListResponse>().await?)
}
```

### `src/yt/videos.rs`
```rust
use super::types::{VideosListResponse};

pub async fn videos_list(
    api_key: &str,
    ids: &[String],
) -> anyhow::Result<VideosListResponse> {
    let mut url = "https://www.googleapis.com/youtube/v3/videos?part=snippet,contentDetails".to_string();
    url.push_str("&id=");
    url.push_str(&ids.join(","));
    url.push_str("&key="); url.push_str(api_key);

    let resp = reqwest::Client::new()
        .get(&url).send().await?
        .error_for_status()?;
    Ok(resp.json::<VideosListResponse>().await?)
}
```

---

## 12) README Snippet (Setup)

1. Create a Google Cloud project → enable **YouTube Data API v3**.
2. Create an **API key** (restrict to `youtube.googleapis.com` if you want).
3. Put the key into the app’s **Settings**.
4. Choose **Today / 48h / 7d** and a **My Search** preset → hit **Search**.
5. Click a result to open in your browser.

> Optional: later add OAuth if you want precise `captions.list` filtering for English tracks.

---

## 13) Definition of Done

- Search limited by **subject** (terms/channel/category), **strict date window**, **English‑leaning**, **no Shorts**.
- **Single** and **Any** run modes; presets import/export.
- Stable UI, no panics, builds on Windows & Linux.

Happy vibing ✨
