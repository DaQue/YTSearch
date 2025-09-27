use super::types::SearchListResponse;
use anyhow::{bail, Context};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
struct GoogleApiErrorResponse {
    error: GoogleApiError,
}

#[derive(Deserialize, Debug)]
struct GoogleApiError {
    #[allow(dead_code)]
    code: i32,
    message: String,
    #[serde(default)]
    errors: Vec<GoogleApiErrorDetail>,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GoogleApiErrorDetail {
    reason: Option<String>,
    #[allow(dead_code)]
    message: Option<String>,
    #[allow(dead_code)]
    domain: Option<String>,
}

fn format_youtube_error(status: reqwest::StatusCode, body: &str, endpoint: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<GoogleApiErrorResponse>(body) {
        let reason = parsed
            .error
            .errors
            .first()
            .and_then(|e| e.reason.as_deref())
            .unwrap_or("");
        let status_str = parsed.error.status.unwrap_or_default();
        if reason.is_empty() && status_str.is_empty() {
            return format!(
                "YouTube {} failed (HTTP {}): {}",
                endpoint,
                status.as_u16(),
                parsed.error.message
            );
        }
        return format!(
            "YouTube {} failed (HTTP {}, {}{}): {}",
            endpoint,
            status.as_u16(),
            status_str,
            if reason.is_empty() {
                "".into()
            } else {
                format!(", reason={}", reason)
            },
            parsed.error.message
        );
    }
    // Fallback: raw body
    format!(
        "YouTube {} failed (HTTP {}): {}",
        endpoint,
        status.as_u16(),
        body.trim()
    )
}

fn parse_error_reason(body: &str) -> Option<String> {
    if let Ok(parsed) = serde_json::from_str::<GoogleApiErrorResponse>(body) {
        return parsed.error.errors.first().and_then(|e| e.reason.clone());
    }
    None
}

fn load_alt_keys(current: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let current_trimmed = current.trim();

    for fname in ["YT_API_private.alt", "YT_API_private,old", "YT_API_private"] {
        if let Ok(contents) = fs::read_to_string(fname) {
            let trimmed = contents.trim().to_owned();
            if !trimmed.is_empty() && trimmed != current_trimmed {
                keys.push(trimmed);
            }
        }
    }
    keys
}

#[allow(dead_code)]
pub async fn search_list(
    api_key: &str,
    params: &[(&str, String)],
) -> anyhow::Result<SearchListResponse> {
    let mut url =
        "https://www.googleapis.com/youtube/v3/search?part=snippet&type=video".to_string();
    for (k, v) in params {
        url.push('&');
        url.push_str(k);
        url.push('=');
        url.push_str(&urlencoding::encode(v));
    }
    url.push_str("&key=");
    url.push_str(api_key.trim());

    let client = reqwest::Client::new();
    let mut resp = client.get(&url).send().await?;
    let mut status = resp.status();
    let mut bytes = resp.bytes().await?;
    if !status.is_success() {
        let mut body_string = String::from_utf8_lossy(&bytes).to_string();
        let reason = parse_error_reason(&body_string).unwrap_or_default();
        let is_key_issue = status.as_u16() == 403
            && (reason.contains("quota")
                || reason.contains("dailyLimitExceeded")
                || reason.contains("keyInvalid")
                || reason.contains("forbidden")
                || reason.contains("ipRefererBlocked")
                || reason.contains("accessNotConfigured"));
        if is_key_issue {
            let alt_keys = load_alt_keys(api_key);
            for alt_key in alt_keys {
                let mut alt_url =
                    "https://www.googleapis.com/youtube/v3/search?part=snippet&type=video"
                        .to_string();
                for (k, v) in params {
                    alt_url.push('&');
                    alt_url.push_str(k);
                    alt_url.push('=');
                    alt_url.push_str(&urlencoding::encode(v));
                }
                alt_url.push_str("&key=");
                alt_url.push_str(alt_key.trim());

                resp = client.get(&alt_url).send().await.with_context(|| {
                    "retry with alternate API key failed to send request".to_string()
                })?;
                status = resp.status();
                bytes = resp.bytes().await?;
                if status.is_success() {
                    let parsed = serde_json::from_slice::<SearchListResponse>(&bytes)?;
                    return Ok(parsed);
                }
                // If this alt key also fails, try the next one
            }
        }
        body_string = String::from_utf8_lossy(&bytes).to_string();
        bail!(format_youtube_error(status, &body_string, "search.list"));
    }
    let parsed = serde_json::from_slice::<SearchListResponse>(&bytes)?;
    Ok(parsed)
}
