use super::types::VideosListResponse;

#[allow(dead_code)]
pub async fn videos_list(api_key: &str, ids: &[String]) -> anyhow::Result<VideosListResponse> {
    if ids.is_empty() {
        return Ok(VideosListResponse { items: vec![] });
    }
    let mut url =
        "https://www.googleapis.com/youtube/v3/videos?part=snippet,contentDetails".to_string();
    url.push_str("&id=");
    url.push_str(&ids.join(","));
    url.push_str("&key=");
    url.push_str(api_key);

    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .error_for_status()?;
    let parsed = resp.json::<VideosListResponse>().await?;
    Ok(parsed)
}
