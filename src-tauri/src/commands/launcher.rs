use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub comment: Option<String>,
    pub desktop_file: String,
    pub keywords: Vec<String>,
}

fn parse_bool_field(fields: &HashMap<String, String>, key: &str) -> bool {
    fields
        .get(key)
        .map(|value| value.eq_ignore_ascii_case("true") || value == "1")
        .unwrap_or(false)
}

/// Parse a single .desktop file into an AppEntry
fn parse_desktop_file(path: &PathBuf) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut fields: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') && line != "[Desktop Entry]" {
            in_desktop_entry = false;
            continue;
        }
        if !in_desktop_entry || line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            fields.insert(key.trim().to_string(), val.trim().to_string());
        }
    }

    if parse_bool_field(&fields, "NoDisplay") {
        return None;
    }
    if fields.get("Type").map(|s| s != "Application").unwrap_or(true) {
        return None;
    }
    if parse_bool_field(&fields, "Hidden") {
        return None;
    }

    let name = fields.get("Name")?.clone();
    let exec_raw = fields.get("Exec")?.clone();
    let exec = exec_raw
        .split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");

    let categories = fields
        .get("Categories")
        .map(|s| {
            s.split(';')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let keywords = fields
        .get("Keywords")
        .map(|s| {
            s.split(';')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    Some(AppEntry {
        name,
        exec,
        icon: fields.get("Icon").cloned(),
        categories,
        comment: fields.get("Comment").cloned(),
        desktop_file: path.to_string_lossy().to_string(),
        keywords,
    })
}

/// Scan standard XDG application directories for .desktop files
fn desktop_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];

    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".local/share/applications"));
    }

    if let Ok(xdg) = std::env::var("XDG_DATA_DIRS") {
        for dir in xdg.split(':') {
            dirs.push(PathBuf::from(dir).join("applications"));
        }
    }

    dirs
}

#[tauri::command]
pub fn list_apps() -> Result<Vec<AppEntry>, String> {
    let mut apps = Vec::new();
    let mut seen = HashSet::new();

    for dir in desktop_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }

            let fname = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if seen.contains(&fname) {
                continue;
            }
            seen.insert(fname);

            if let Some(app) = parse_desktop_file(&path) {
                apps.push(app);
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(apps)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSearchResult {
    pub app: AppEntry,
    pub score: i64,
}

#[tauri::command]
pub fn search_apps(query: String) -> Result<Vec<AppSearchResult>, String> {
    let apps = list_apps()?;
    if query.trim().is_empty() {
        return Ok(apps
            .into_iter()
            .take(12)
            .map(|app| AppSearchResult { score: 0, app })
            .collect());
    }

    let matcher = SkimMatcherV2::default();
    let mut results: Vec<AppSearchResult> = apps
        .into_iter()
        .filter_map(|app| {
            let score = [
                matcher.fuzzy_match(&app.name, &query).map(|s| s * 3),
                app.comment
                    .as_deref()
                    .and_then(|comment| matcher.fuzzy_match(comment, &query)),
                app.keywords
                    .iter()
                    .filter_map(|keyword| matcher.fuzzy_match(keyword, &query))
                    .max(),
                app.categories
                    .iter()
                    .filter_map(|category| matcher.fuzzy_match(category, &query))
                    .max(),
            ]
            .into_iter()
            .flatten()
            .max()?;

            Some(AppSearchResult { app, score })
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(20);
    Ok(results)
}

#[tauri::command]
pub fn launch_app(exec: String) -> Result<(), String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(&exec)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_hyprland_launcher_bind() -> String {
    "# Add to ~/.config/hypr/hyprland.conf:\nbind = SUPER, SUPER, exec, edex-de --launcher\nbind = , catchall, exec, edex-de --launcher".to_string()
}
