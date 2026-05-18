use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mime: Option<String>,
    pub modified: Option<u64>,
}

#[tauri::command]
pub fn list_dir(path: String, show_dotfiles: bool) -> Result<Vec<FileEntry>, String> {
    let dir = Path::new(&path);
    if !dir.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    let mut entries = vec![];
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_dotfiles && name.starts_with('.') {
            continue;
        }

        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let is_dir = meta.is_dir();
        let mime = if !is_dir {
            Some(
                mime_guess::from_path(entry.path())
                    .first_or_octet_stream()
                    .to_string(),
            )
        } else {
            None
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        entries.push(FileEntry {
            name,
            path: entry.path().to_string_lossy().to_string(),
            is_dir,
            size: meta.len(),
            mime,
            modified,
        });
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_entry(from: String, to: String) -> Result<(), String> {
    std::fs::rename(&from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_entry(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(p).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn create_directory(path: String) -> Result<(), String> {
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct FuzzyMatch {
    pub path: String,
    pub name: String,
    pub score: i64,
}

#[tauri::command]
pub fn fuzzy_search_files(cwd: String, query: String) -> Result<Vec<FuzzyMatch>, String> {
    let matcher = SkimMatcherV2::default();
    let mut results = vec![];

    for entry in walkdir::WalkDir::new(&cwd)
        .max_depth(5)
        .into_iter()
        .filter_map(Result::ok)
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(score) = matcher.fuzzy_match(&name, &query) {
            results.push(FuzzyMatch {
                path: entry.path().to_string_lossy().to_string(),
                name,
                score,
            });
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(50);
    Ok(results)
}
