use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use directories::ProjectDirs;
use egui::{self, ColorImage, Context, ImageData, TextureHandle, TextureOptions, Vec2};
use tokio::runtime::Runtime;

pub const MAX_THUMB_WIDTH: f32 = 160.0;
pub const MAX_THUMB_HEIGHT: f32 = 90.0;

pub struct ThumbnailCache {
    entries: HashMap<String, ThumbnailEntry>,
    client: reqwest::Client,
    tx: Sender<ThumbnailMessage>,
    rx: Receiver<ThumbnailMessage>,
    disk_dir: PathBuf,
}

struct ThumbnailEntry {
    url: Option<String>,
    state: ThumbnailState,
}

enum ThumbnailState {
    Idle,
    Missing,
    Loading,
    Ready { texture: TextureHandle, size: Vec2 },
    Failed,
}

pub struct ThumbnailRef {
    pub texture: TextureHandle,
    pub original_size: Vec2,
    pub display_size: Vec2,
}

struct ThumbnailMessage {
    video_id: String,
    url: String,
    payload: Result<ThumbnailPayload, String>,
}

struct ThumbnailPayload {
    image: ColorImage,
    bytes: Vec<u8>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let disk_dir = ProjectDirs::from("com", "yourname", "YTSearch")
            .map(|proj| proj.config_dir().join("thumbnails"))
            .unwrap_or_else(|| PathBuf::from("thumbnails"));
        if let Err(err) = fs::create_dir_all(&disk_dir) {
            eprintln!("Failed to create thumbnail cache dir: {err}");
        }
        Self {
            entries: HashMap::new(),
            client: reqwest::Client::new(),
            tx,
            rx,
            disk_dir,
        }
    }

    pub fn retain_ids<'a, I>(&mut self, ids: I)
    where
        I: IntoIterator<Item = &'a str>,
    {
        let keep: HashSet<String> = ids.into_iter().map(|id| id.to_owned()).collect();
        self.entries.retain(|id, _| keep.contains(id));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn request(&mut self, video_id: &str, url: Option<&str>, ctx: &Context, runtime: &Runtime) {
        let entry = self
            .entries
            .entry(video_id.to_owned())
            .or_insert_with(|| ThumbnailEntry {
                url: None,
                state: ThumbnailState::Idle,
            });

        match url {
            Some(actual) if !actual.is_empty() => {
                let url_has_changed = entry.url.as_deref() != Some(actual);
                let needs_fetch = matches!(
                    entry.state,
                    ThumbnailState::Idle | ThumbnailState::Failed | ThumbnailState::Missing
                );
                if matches!(entry.state, ThumbnailState::Idle) {
                    if let Some(cached) = load_from_disk(&self.disk_dir, video_id, actual) {
                        let [w, h] = cached.size;
                        let original = Vec2::new(w as f32, h as f32);
                        let texture = ctx.load_texture(
                            format!("thumbnail://{}", video_id),
                            ImageData::from(cached),
                            TextureOptions::LINEAR,
                        );
                        entry.url = Some(actual.to_owned());
                        entry.state = ThumbnailState::Ready {
                            texture,
                            size: original,
                        };
                        return;
                    }
                }
                if url_has_changed || needs_fetch {
                    entry.url = Some(actual.to_owned());
                    entry.state = ThumbnailState::Loading;
                    ctx.request_repaint();

                    let tx = self.tx.clone();
                    let client = self.client.clone();
                    let video_id_owned = video_id.to_owned();
                    let url_owned = actual.to_owned();
                    runtime.spawn(async move {
                        let payload = fetch_thumbnail(client, &url_owned).await;
                        let _ = tx.send(ThumbnailMessage {
                            video_id: video_id_owned,
                            url: url_owned,
                            payload,
                        });
                    });
                }
            }
            _ => {
                entry.url = None;
                entry.state = ThumbnailState::Missing;
            }
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        while let Ok(message) = self.rx.try_recv() {
            if let Some(entry) = self.entries.get_mut(&message.video_id) {
                if entry.url.as_deref() != Some(message.url.as_str()) {
                    continue;
                }
                match message.payload {
                    Ok(payload) => {
                        let [w, h] = payload.image.size;
                        let original = Vec2::new(w as f32, h as f32);
                        let image_data = ImageData::from(payload.image);
                        match &mut entry.state {
                            ThumbnailState::Ready { texture, size } => {
                                texture.set(image_data, TextureOptions::LINEAR);
                                *size = original;
                            }
                            _ => {
                                let texture = ctx.load_texture(
                                    format!("thumbnail://{}", message.video_id),
                                    image_data,
                                    TextureOptions::LINEAR,
                                );
                                entry.state = ThumbnailState::Ready {
                                    texture,
                                    size: original,
                                };
                            }
                        }
                        if let Err(err) = persist_to_disk(
                            &self.disk_dir,
                            &message.video_id,
                            &message.url,
                            &payload.bytes,
                        ) {
                            eprintln!("Failed to persist thumbnail: {err}");
                        }
                    }
                    Err(_) => {
                        entry.state = ThumbnailState::Failed;
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    pub fn thumbnail(&self, video_id: &str) -> Option<ThumbnailRef> {
        let entry = self.entries.get(video_id)?;
        if let ThumbnailState::Ready { texture, size } = &entry.state {
            let display = scaled_size(*size);
            Some(ThumbnailRef {
                texture: texture.clone(),
                original_size: *size,
                display_size: display,
            })
        } else {
            None
        }
    }

    pub fn is_loading(&self, video_id: &str) -> bool {
        matches!(
            self.entries.get(video_id).map(|entry| &entry.state),
            Some(ThumbnailState::Loading)
        )
    }

    pub fn is_failed(&self, video_id: &str) -> bool {
        matches!(
            self.entries.get(video_id).map(|entry| &entry.state),
            Some(ThumbnailState::Failed)
        )
    }
}
fn scaled_size(original: Vec2) -> Vec2 {
    if original.x <= MAX_THUMB_WIDTH && original.y <= MAX_THUMB_HEIGHT {
        return original;
    }
    let width_ratio = MAX_THUMB_WIDTH / original.x;
    let height_ratio = MAX_THUMB_HEIGHT / original.y;
    let scale = width_ratio.min(height_ratio);
    Vec2::new(original.x * scale, original.y * scale)
}

async fn fetch_thumbnail(client: reqwest::Client, url: &str) -> Result<ThumbnailPayload, String> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| err.to_string())?;
    let bytes = response.bytes().await.map_err(|err| err.to_string())?;
    let buffer = bytes.to_vec();
    let image = decode_image(&buffer)?;
    Ok(ThumbnailPayload {
        image,
        bytes: buffer,
    })
}

fn decode_image(bytes: &[u8]) -> Result<ColorImage, String> {
    let image = image::load_from_memory(bytes).map_err(|err| err.to_string())?;
    let image = image.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_vec();
    Ok(ColorImage::from_rgba_unmultiplied(size, &pixels))
}

fn cache_paths(base: &Path, video_id: &str) -> (PathBuf, PathBuf) {
    let sanitized = sanitize_id(video_id);
    let image_path = base.join(format!("{sanitized}.bin"));
    let url_path = base.join(format!("{sanitized}.url"));
    (image_path, url_path)
}

fn load_from_disk(base: &Path, video_id: &str, url: &str) -> Option<ColorImage> {
    let (image_path, url_path) = cache_paths(base, video_id);
    let stored_url = fs::read_to_string(url_path).ok()?;
    if stored_url.trim() != url {
        return None;
    }
    let bytes = fs::read(image_path).ok()?;
    decode_image(&bytes).ok()
}

fn persist_to_disk(base: &Path, video_id: &str, url: &str, bytes: &[u8]) -> std::io::Result<()> {
    fs::create_dir_all(base)?;
    let (image_path, url_path) = cache_paths(base, video_id);
    fs::write(&image_path, bytes)?;
    fs::write(&url_path, url)?;
    Ok(())
}

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect()
}
