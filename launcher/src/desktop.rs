use std::{path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub comment: Option<String>,
    pub no_display: bool,
    pub terminal: bool,
}

pub fn scan_applications() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let search_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        dirs_home_apps(),
    ];

    for dir in search_dirs {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|ext| ext == "desktop") {
                    if let Some(app) = parse_desktop_file(&entry.path()) {
                        if !app.no_display && !app.exec.is_empty() {
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }

    apps.sort_by_key(|app| app.name.to_lowercase());
    apps.dedup_by(|a, b| a.name == b.name);
    apps
}

fn dirs_home_apps() -> PathBuf {
    std::env::var("HOME")
        .map(|home| PathBuf::from(home).join(".local/share/applications"))
        .unwrap_or_else(|_| PathBuf::from("/"))
}

fn parse_desktop_file(path: &Path) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut categories = Vec::new();
    let mut comment = None;
    let mut no_display = false;
    let mut terminal = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') {
            in_desktop_entry = false;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "Name" => name = Some(value.trim().to_string()),
                "Exec" => exec = Some(clean_exec(value)),
                "Icon" => icon = Some(value.trim().to_string()),
                "Categories" => {
                    categories = value
                        .split(';')
                        .filter(|entry| !entry.is_empty())
                        .map(|entry| entry.to_string())
                        .collect();
                }
                "Comment" => comment = Some(value.trim().to_string()),
                "NoDisplay" => no_display = value.trim().eq_ignore_ascii_case("true"),
                "Terminal" => terminal = value.trim().eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    Some(AppEntry {
        name: name?,
        exec: exec?,
        icon,
        categories,
        comment,
        no_display,
        terminal,
    })
}

fn clean_exec(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|segment| !segment.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
}
