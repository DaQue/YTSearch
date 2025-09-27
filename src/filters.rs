use crate::prefs::{GlobalPrefs, MySearch};
use crate::yt::types::VideoDetails;

#[allow(dead_code)]
pub fn parse_iso8601_duration(s: &str) -> Option<u64> {
    // Simple parser for PT#H#M#S (expand as needed)
    let (mut h, mut m, mut sec) = (0u64, 0u64, 0u64);
    if !s.starts_with('P') {
        return None;
    }
    let t = s.split('T').nth(1)?;
    let mut num = String::new();
    for ch in t.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
            continue;
        }
        let val: u64 = num.parse().ok()?;
        num.clear();
        match ch {
            'H' => h = val,
            'M' => m = val,
            'S' => sec = val,
            _ => {}
        }
    }
    Some(h * 3600 + m * 60 + sec)
}

#[allow(dead_code)]
pub fn contains_any(hay: &str, needles: &[String]) -> bool {
    let h = hay.to_ascii_lowercase();
    needles
        .iter()
        .map(|needle| needle.trim())
        .filter(|needle| !needle.is_empty())
        .any(|needle| h.contains(&needle.to_ascii_lowercase()))
}

pub fn matches_post_filters(
    video: &VideoDetails,
    prefs: &GlobalPrefs,
    search: &MySearch,
    blocked_channels: &[String],
) -> bool {
    let min_secs = search
        .min_duration_override
        .unwrap_or(prefs.min_duration_secs) as u64;
    if video.duration_secs < min_secs {
        return false;
    }

    let want_en = search.english_only_override.unwrap_or(prefs.english_only);
    if want_en {
        let lang_ok = language_is_english(video.default_audio_lang.as_deref())
            || language_is_english(video.default_lang.as_deref())
            || video.has_caption_lang_en.unwrap_or(false)
            || looks_english(&video.title_lower);
        if !lang_ok {
            return false;
        }
    }

    if contains_any(&video.title_lower, &search.query.not_terms) {
        return false;
    }

    if matches_channel(
        &video.channel_handle,
        &video.channel_title,
        blocked_channels,
    ) {
        return false;
    }

    if !search.query.channel_deny.is_empty()
        && matches_channel(
            &video.channel_handle,
            &video.channel_title,
            &search.query.channel_deny,
        )
    {
        return false;
    }

    if !search.query.channel_allow.is_empty()
        && !matches_channel(
            &video.channel_handle,
            &video.channel_title,
            &search.query.channel_allow,
        )
    {
        return false;
    }

    true
}

fn language_is_english(code: Option<&str>) -> bool {
    code.map(|c| c.to_ascii_lowercase())
        .map(|lower| lower.starts_with("en"))
        .unwrap_or(false)
}

pub fn matches_channel(handle: &str, title: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    let handle = handle.to_ascii_lowercase();
    let title = title.to_ascii_lowercase();

    patterns
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .any(|pattern| {
            let cleaned = pattern.trim_start_matches('@').to_ascii_lowercase();
            handle == cleaned || title == cleaned || title.contains(&cleaned)
        })
}

fn looks_english(text: &str) -> bool {
    let mut total = 0usize;
    let mut asciiish = 0usize;
    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }
        total += 1;
        if ch.is_ascii_alphabetic() {
            asciiish += 1;
            continue;
        }
        if matches!(
            ch,
            '-' | '_' | ':' | '!' | '?' | ',' | '.' | ';' | '\'' | '"' | '/' | '(' | ')' | '#'
        ) {
            asciiish += 1;
        }
    }
    if total == 0 {
        return true;
    }
    asciiish * 100 / total >= 60
}
