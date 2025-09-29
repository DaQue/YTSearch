#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct VideoDetails {
    pub id: String,
    pub title: String,
    pub title_lower: String,
    pub channel_title: String,
    pub channel_handle: String,
    pub channel_display_name: Option<String>,
    pub channel_custom_url: Option<String>,
    pub published_at: String,
    pub duration_secs: u64,
    pub default_audio_lang: Option<String>,
    pub default_lang: Option<String>,
    pub thumbnail_url: Option<String>,
    pub url: String,
    pub has_caption_lang_en: Option<bool>,
    pub source_presets: Vec<String>,
}

#[derive(Deserialize)]
pub struct SearchListResponse {
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    pub items: Vec<SearchItem>,
}
#[derive(Deserialize)]
pub struct SearchItem {
    pub id: SearchId,
    pub snippet: Snippet,
}
#[derive(Deserialize)]
pub struct SearchId {
    #[serde(rename = "videoId")]
    pub video_id: Option<String>,
}
#[derive(Deserialize)]
pub struct Snippet {
    #[serde(rename = "publishedAt")]
    pub published_at: String,
}

#[derive(Deserialize)]
pub struct VideosListResponse {
    pub items: Vec<VideoItem>,
}
#[derive(Deserialize)]
pub struct VideoItem {
    pub id: String,
    pub snippet: VideoSnippet,
    #[serde(rename = "contentDetails")]
    pub content_details: ContentDetails,
}
#[derive(Deserialize)]
pub struct VideoSnippet {
    pub title: String,
    #[serde(rename = "channelTitle")]
    pub channel_title: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    #[serde(rename = "publishedAt")]
    pub published_at: String,
    #[serde(rename = "defaultAudioLanguage")]
    pub default_audio_language: Option<String>,
    #[serde(rename = "defaultLanguage")]
    pub default_language: Option<String>,
    pub thumbnails: Option<Thumbs>,
}
#[derive(Deserialize)]
pub struct Thumbs {
    #[serde(rename = "medium")]
    pub medium: Option<Thumb>,
}
#[derive(Deserialize)]
pub struct Thumb {
    pub url: String,
}
#[derive(Deserialize)]
pub struct ContentDetails {
    pub duration: String,
}

#[derive(Deserialize)]
pub struct ChannelsListResponse {
    pub items: Vec<ChannelItem>,
}

#[derive(Deserialize)]
pub struct ChannelItem {
    pub id: String,
    pub snippet: ChannelSnippet,
}

#[derive(Deserialize)]
pub struct ChannelSnippet {
    pub title: String,
    #[serde(rename = "customUrl")]
    pub custom_url: Option<String>,
}
