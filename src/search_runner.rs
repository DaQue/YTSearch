use std::collections::{HashMap, HashSet};

use anyhow::{Result, bail};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::filters;
use crate::prefs::{self, GlobalPrefs, MySearch, Prefs, QuerySpec, TimeWindow, TimeWindowPreset};
use crate::yt::{
    channels, search,
    types::{SearchListResponse, VideoDetails, VideoItem},
    videos,
};
use anyhow::Context;
use std::env;

const DEFAULT_MAX_SEARCH_PAGES: usize = 4;

fn max_search_pages() -> usize {
    match env::var("YTSEARCH_MAX_SEARCH_PAGES") {
        Ok(val) => val
            .trim()
            .parse::<usize>()
            .ok()
            .filter(|n| (1..=10).contains(n))
            .unwrap_or(DEFAULT_MAX_SEARCH_PAGES),
        Err(_) => DEFAULT_MAX_SEARCH_PAGES,
    }
}

pub enum RunMode {
    Any,
    Single(String),
}

pub struct SearchOutcome {
    pub videos: Vec<VideoDetails>,
    pub presets_ran: usize,
    pub pages_fetched: usize,
    pub duplicates_within_presets: usize,
    pub duplicates_across_presets: usize,
    pub raw_items: usize,
    pub unique_ids: usize,
    pub passed_filters: usize,
}

struct SingleSearchOutcome {
    videos: Vec<VideoDetails>,
    pages_fetched: usize,
    duplicates_within: usize,
    raw_items: usize,
    unique_ids: usize,
}

pub async fn run_searches(prefs: Prefs, mode: RunMode) -> Result<SearchOutcome> {
    let Prefs {
        api_key,
        mut global,
        searches,
        blocked_channels,
    } = prefs;

    prefs::normalize_duration_filters(&mut global);

    let api_key = api_key.trim().to_owned();
    if api_key.is_empty() {
        bail!("Set your YouTube Data API key in the settings panel first.");
    }

    if searches.is_empty() {
        bail!("No searches configured. Add a preset in the settings panel.");
    }

    let (targets, is_any_mode): (Vec<MySearch>, bool) = match mode {
        RunMode::Any => {
            let enabled: Vec<MySearch> = searches.into_iter().filter(|s| s.enabled).collect();
            if enabled.is_empty() {
                bail!("Enable at least one preset before running in Any mode.");
            }
            (enabled, true)
        }
        RunMode::Single(selected_id) => {
            let mut iter = searches.into_iter();
            if let Some(search) = iter.find(|s| s.id == selected_id) {
                (vec![search], false)
            } else {
                bail!("Preset '{}' not found.", selected_id);
            }
        }
    };

    let blocked_keys = prefs::blocked_keys(&blocked_channels);

    let mut index_by_id: HashMap<String, usize> = HashMap::new();
    let mut aggregated: Vec<VideoDetails> = Vec::new();
    let mut total_pages = 0usize;
    let mut presets_ran = 0usize;
    let mut duplicates_within_presets = 0usize;
    let mut duplicates_across_presets = 0usize;
    let mut total_raw_items = 0usize;
    let mut total_unique_ids = 0usize;
    let mut total_passed_filters = 0usize;

    for search in targets {
        let outcome = run_single_search(&api_key, &global, &search, &blocked_keys).await?;
        presets_ran += 1;
        total_pages += outcome.pages_fetched;
        duplicates_within_presets += outcome.duplicates_within;
        total_raw_items += outcome.raw_items;
        total_unique_ids += outcome.unique_ids;

        let mut videos = outcome.videos;
        total_passed_filters += videos.len();

        if is_any_mode {
            for video in videos.drain(..) {
                if let Some(idx) = index_by_id.get(&video.id).copied() {
                    let existing = &mut aggregated[idx];
                    let new_sources = video.source_presets.clone();
                    for source in new_sources {
                        if !existing.source_presets.iter().any(|s| s == &source) {
                            existing.source_presets.push(source);
                        }
                    }
                    duplicates_across_presets += 1;
                } else {
                    let idx = aggregated.len();
                    index_by_id.insert(video.id.clone(), idx);
                    aggregated.push(video);
                }
            }
        } else {
            aggregated.append(&mut videos);
        }
    }

    aggregated.sort_by(|a, b| b.published_at.cmp(&a.published_at));

    Ok(SearchOutcome {
        videos: aggregated,
        presets_ran,
        pages_fetched: total_pages,
        duplicates_within_presets,
        duplicates_across_presets,
        raw_items: total_raw_items,
        unique_ids: total_unique_ids,
        passed_filters: total_passed_filters,
    })
}

async fn run_single_search(
    api_key: &str,
    global: &GlobalPrefs,
    search: &MySearch,
    blocked_keys: &[String],
) -> Result<SingleSearchOutcome> {
    let mut base_params = build_query_params(global, search)?;
    if let Some(window) = resolve_window(global, search) {
        base_params.push(("publishedAfter", window.start_rfc3339.clone()));
        base_params.push(("publishedBefore", window.end_rfc3339.clone()));
    }
    base_params.push(("order", "date".to_owned()));
    base_params.push(("maxResults", "25".to_owned()));

    let mut page_token: Option<String> = None;
    let mut pages_fetched = 0usize;
    let mut duplicates_within = 0usize;
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut collected: Vec<VideoDetails> = Vec::new();
    let mut raw_items_total = 0usize;
    let mut unique_ids_total = 0usize;

    while pages_fetched < max_search_pages() {
        let mut params = base_params.clone();
        if let Some(token) = &page_token {
            params.push(("pageToken", token.clone()));
        }

        let response = search::search_list(api_key, &params)
            .await
            .with_context(|| "search.list failed — check API key, quotas, or restrictions")?;
        pages_fetched += 1;

        let SearchListResponse {
            next_page_token,
            items,
        } = response;
        raw_items_total += items.len();
        let mut request_ids: Vec<String> = Vec::new();
        for item in items {
            if let Some(video_id) = item.id.video_id {
                if seen_ids.insert(video_id.clone()) {
                    request_ids.push(video_id);
                } else {
                    duplicates_within += 1;
                }
            }
        }
        unique_ids_total += request_ids.len();
        if !request_ids.is_empty() {
            let videos = videos::videos_list(api_key, &request_ids)
                .await
                .with_context(|| "videos.list failed — check API key, quotas, or restrictions")?;
            for video in videos.items {
                let mut details = map_video_item(video);
                if filters::matches_post_filters(&details, global, search, blocked_keys) {
                    details.source_presets.push(search.name.clone());
                    collected.push(details);
                }
            }
        }

        match next_page_token {
            Some(token) => {
                page_token = Some(token);
            }
            None => break,
        }
    }

    if !collected.is_empty() {
        enhance_channel_metadata(api_key, &mut collected).await;
    }

    Ok(SingleSearchOutcome {
        videos: collected,
        pages_fetched,
        duplicates_within,
        raw_items: raw_items_total,
        unique_ids: unique_ids_total,
    })
}

async fn enhance_channel_metadata(api_key: &str, videos: &mut [VideoDetails]) {
    let mut ids: Vec<String> = videos
        .iter()
        .map(|v| v.channel_handle.clone())
        .filter(|id| !id.trim().is_empty())
        .collect();
    ids.sort();
    ids.dedup();

    if ids.is_empty() {
        for video in videos.iter_mut() {
            if video.channel_display_name.is_none() && !video.channel_title.trim().is_empty() {
                video.channel_display_name = Some(video.channel_title.clone());
            }
        }
        return;
    }

    let mut metadata: HashMap<String, (String, Option<String>)> = HashMap::new();
    for chunk in ids.chunks(50) {
        match channels::channels_list(api_key, chunk).await {
            Ok(resp) => {
                for item in resp.items {
                    let title = item.snippet.title.trim().to_string();
                    let custom = item
                        .snippet
                        .custom_url
                        .as_ref()
                        .map(|url| url.trim())
                        .filter(|url| !url.is_empty())
                        .map(|url| {
                            if url.starts_with('@') {
                                url.to_string()
                            } else {
                                format!("@{}", url.trim_start_matches('@'))
                            }
                        });
                    metadata.insert(item.id, (title, custom));
                }
            }
            Err(err) => {
                eprintln!("channels.list request failed: {err}");
            }
        }
    }

    for video in videos.iter_mut() {
        if let Some((title, custom)) = metadata.get(&video.channel_handle) {
            if !title.trim().is_empty() {
                video.channel_display_name = Some(title.clone());
            }
            if let Some(handle) = custom {
                video.channel_custom_url = Some(handle.clone());
            }
        }

        if video.channel_display_name.is_none() && !video.channel_title.trim().is_empty() {
            video.channel_display_name = Some(video.channel_title.clone());
        }

        if video.channel_custom_url.is_none() && video.channel_handle.starts_with('@') {
            video.channel_custom_url = Some(video.channel_handle.clone());
        }
    }
}

pub fn resolve_window(global: &GlobalPrefs, search: &MySearch) -> Option<TimeWindow> {
    if let Some(override_window) = &search.window_override {
        return Some(override_window.clone());
    }

    let preset = global.default_window;
    window_for_preset(preset)
}

fn window_for_preset(preset: TimeWindowPreset) -> Option<TimeWindow> {
    let now = OffsetDateTime::now_utc();
    let (start, end) = match preset {
        TimeWindowPreset::Today => Some((now - Duration::days(1), now)),
        TimeWindowPreset::H48 => Some((now - Duration::hours(48), now)),
        TimeWindowPreset::D7 => Some((now - Duration::days(7), now)),
        TimeWindowPreset::AllTime => None,
    }?;

    let start = start
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned());
    let end = end
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned());

    Some(TimeWindow {
        start_rfc3339: start,
        end_rfc3339: end,
    })
}

pub fn build_query_params(
    global: &GlobalPrefs,
    search: &MySearch,
) -> Result<Vec<(&'static str, String)>> {
    let mut params = Vec::new();
    let query_text = build_query_text(&search.query);

    if query_text.trim().is_empty() {
        bail!("Search query is empty. Add some terms to your preset.");
    }
    params.push(("q", query_text));

    if let Some(category_id) = search.query.category_id {
        params.push(("videoCategoryId", category_id.to_string()));
    }

    if let Some(region) = global.region_code.as_ref() {
        params.push(("regionCode", region.clone()));
    }

    let require_captions = search
        .require_captions_override
        .unwrap_or(global.require_captions);
    if require_captions {
        params.push(("videoCaption", "closedCaption".to_owned()));
    }

    let min_duration = search
        .min_duration_override
        .unwrap_or(global.min_duration_secs);
    if min_duration >= 1200 {
        params.push(("videoDuration", "long".to_owned()));
    } else if min_duration >= 600 {
        params.push(("videoDuration", "medium".to_owned()));
    }

    Ok(params)
}

fn build_query_text(spec: &QuerySpec) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(q) = &spec.q {
        let q = q.trim();
        if !q.is_empty() {
            parts.push(q.to_owned());
        }
    }

    let any_terms: Vec<String> = spec
        .any_terms
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(format_query_token)
        .collect();
    if !any_terms.is_empty() {
        parts.push(format!("({})", any_terms.join(" OR ")));
    }

    parts.extend(
        spec.all_terms
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(format_query_token),
    );

    for term in spec.not_terms.iter() {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            continue;
        }
        parts.push(format!("-{}", format_query_token(trimmed)));
    }

    parts.join(" ")
}

fn format_query_token(term: &str) -> String {
    if term.is_empty() {
        return String::new();
    }
    let needs_quotes = term.chars().any(|c| c.is_whitespace()) || term.contains('"');
    if needs_quotes {
        let escaped = term.replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        term.to_string()
    }
}

fn map_video_item(item: VideoItem) -> VideoDetails {
    let snippet = item.snippet;
    let content = item.content_details;

    let thumbnail_url = snippet
        .thumbnails
        .as_ref()
        .and_then(|thumbs| thumbs.medium.as_ref())
        .map(|thumb| thumb.url.clone());

    VideoDetails {
        id: item.id.clone(),
        title: snippet.title.clone(),
        title_lower: snippet.title.to_ascii_lowercase(),
        channel_title: snippet.channel_title.clone(),
        channel_handle: snippet.channel_id.clone(),
        channel_display_name: None,
        channel_custom_url: None,
        published_at: snippet.published_at.clone(),
        duration_secs: filters::parse_iso8601_duration(&content.duration).unwrap_or(0),
        default_audio_lang: snippet.default_audio_language.clone(),
        default_lang: snippet.default_language.clone(),
        thumbnail_url,
        url: format!("https://www.youtube.com/watch?v={}", item.id),
        has_caption_lang_en: None,
        source_presets: Vec::new(),
    }
}
