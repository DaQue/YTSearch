use crate::prefs::TimeWindowPreset;

pub fn time_window_label(preset: TimeWindowPreset) -> &'static str {
    match preset {
        TimeWindowPreset::Today => "Today",
        TimeWindowPreset::H48 => "48h",
        TimeWindowPreset::D7 => "7d",
        TimeWindowPreset::AllTime => "Any date",
    }
}

pub fn format_duration(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let mut parts = Vec::new();
    if hours > 0 {
        parts.push(format!("{}h", hours));
    }
    if minutes > 0 || hours > 0 {
        parts.push(format!("{}m", minutes));
    }
    parts.push(format!("{}s", seconds));

    parts.join(" ")
}

pub fn open_in_browser(url: &str) -> Result<(), String> {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        match try_launch_new_window(url) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                return open::that(url)
                    .map(|_| ())
                    .map_err(|e| format!("{err}; fallback failed: {e}"));
            }
            Err(_) => {}
        }
    }

    open::that(url).map(|_| ()).map_err(|err| err.to_string())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn try_launch_new_window(url: &str) -> std::io::Result<()> {
    use std::io::ErrorKind;
    use std::process::Command;

    const CANDIDATES: [&str; 4] = [
        "google-chrome",
        "chromium",
        "brave-browser",
        "microsoft-edge",
    ];

    for cmd in CANDIDATES {
        match Command::new(cmd).arg("--new-window").arg(url).spawn() {
            Ok(_) => return Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        }
    }

    Err(std::io::Error::new(
        ErrorKind::NotFound,
        "no supported browser command found",
    ))
}
