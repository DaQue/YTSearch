use super::types::SearchListResponse;

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
    url.push_str(api_key);

    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .error_for_status()?;
    let parsed = resp.json::<SearchListResponse>().await?;
    Ok(parsed)
}
