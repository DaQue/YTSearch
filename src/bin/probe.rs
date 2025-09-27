use clap::Parser;
use time::{Duration, OffsetDateTime};

use YTSearch::prefs::{self, Prefs, TimeWindow, TimeWindowPreset};
use YTSearch::search_runner::{self, RunMode};

#[derive(Parser, Debug)]
#[command(about = "Inspect YTSearch queries from the terminal")]
struct Args {
    /// Run a specific preset by id (defaults to first enabled)
    #[arg(long)]
    preset: Option<String>,

    /// Ignore prefs window and query this many hours back instead
    #[arg(long, value_name = "HOURS")]
    hours: Option<i64>,

    /// Override the region code (use "none" to clear)
    #[arg(long, value_name = "CODE")]
    region: Option<String>,

    /// Disable English-only filtering for this run
    #[arg(long)]
    allow_any_language: bool,

    /// Ignore NOT terms for this run
    #[arg(long)]
    ignore_not_terms: bool,

    /// Override the main free-text query
    #[arg(long, value_name = "TEXT")]
    query: Option<String>,

    /// Minimum duration override in seconds
    #[arg(long, value_name = "SECONDS")]
    min_duration: Option<u32>,

    /// Print the raw params but skip the API calls
    #[arg(long)]
    dry_run: bool,

    /// Limit printed results
    #[arg(long, default_value_t = 10)]
    limit: usize,
}

fn override_window(prefs: &mut Prefs, hours: Option<i64>) {
    if let Some(hours) = hours {
        let now = OffsetDateTime::now_utc();
        let start = now - Duration::hours(hours);
        let window = TimeWindow {
            start_rfc3339: start
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
            end_rfc3339: now
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        };
        for search in &mut prefs.searches {
            search.window_override = Some(window.clone());
        }
        prefs.global.default_window = TimeWindowPreset::Custom;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut prefs = prefs::load_or_default();
    prefs::add_missing_defaults(&mut prefs);
    prefs.blocked_channels = prefs
        .blocked_channels
        .into_iter()
        .map(|c| c.trim().to_ascii_lowercase())
        .filter(|c| !c.is_empty())
        .collect();
    prefs.blocked_channels.sort();
    prefs.blocked_channels.dedup();
    if prefs.api_key.trim().is_empty() {
        if let Ok(contents) = std::fs::read_to_string("YT_API_private") {
            let trimmed = contents.trim();
            if !trimmed.is_empty() {
                prefs.api_key = trimmed.to_owned();
            }
        }
    }
    if prefs.api_key.trim().is_empty() {
        anyhow::bail!("API key missing in prefs.json and YT_API_private");
    }
    if prefs.searches.is_empty() {
        anyhow::bail!("No presets configured in prefs.json");
    }

    override_window(&mut prefs, args.hours);

    if let Some(region) = args.region.as_ref().map(|s| s.trim()) {
        if region.eq_ignore_ascii_case("none") || region.is_empty() {
            prefs.global.region_code = None;
        } else {
            prefs.global.region_code = Some(region.to_uppercase());
        }
    }

    for search in &mut prefs.searches {
        if args.allow_any_language {
            search.english_only_override = Some(false);
        }
        if args.ignore_not_terms {
            search.query.not_terms.clear();
        }
        if let Some(q) = args.query.as_ref() {
            search.query.q = Some(q.clone());
        }
        if let Some(min) = args.min_duration {
            search.min_duration_override = Some(min);
        }
    }

    let mode = if let Some(id) = args.preset.clone() {
        RunMode::Single(id)
    } else {
        RunMode::Any
    };

    if args.dry_run {
        for search in &prefs.searches {
            let pref_global = &prefs.global;
            let mut params = search_runner::build_query_params(pref_global, search)?;
            let window = search_runner::resolve_window(pref_global, search);
            params.push(("publishedAfter", window.start_rfc3339.clone()));
            params.push(("publishedBefore", window.end_rfc3339.clone()));
            println!("{} => {:?}", search.name, params);
        }
        return Ok(());
    }

    match search_runner::run_searches(prefs, mode).await {
        Ok(outcome) => {
            println!(
                "presets: {} pages: {} raw: {} unique: {} passed: {} kept: {} duplicates: {}",
                outcome.presets_ran,
                outcome.pages_fetched,
                outcome.raw_items,
                outcome.unique_ids,
                outcome.passed_filters,
                outcome.videos.len(),
                outcome.duplicates_within_presets + outcome.duplicates_across_presets,
            );
            for video in outcome.videos.iter().take(args.limit) {
                println!(
                    "{} | {:>4}s | {} | {}",
                    video.published_at,
                    video.duration_secs,
                    video.source_presets.join("+"),
                    video.title
                );
            }
        }
        Err(err) => {
            eprintln!("Error: {err:?}");
        }
    }

    Ok(())
}
