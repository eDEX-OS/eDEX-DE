//! XDG desktop entry scanning per the Desktop Entry Specification.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppEntry {
    /// Desktop file id (relative path with `/` → `-`, without `.desktop`).
    pub id: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub exec: String,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub comment: Option<String>,
    pub terminal: bool,
    pub path: Option<String>,
    pub startup_wm_class: Option<String>,
    pub file: PathBuf,
}

impl AppEntry {
    /// The primary category for display.
    pub fn category(&self) -> Option<String> {
        const MAIN: [&str; 13] = [
            "AudioVideo",
            "Audio",
            "Video",
            "Development",
            "Education",
            "Game",
            "Graphics",
            "Network",
            "Office",
            "Science",
            "Settings",
            "System",
            "Utility",
        ];
        self.categories
            .iter()
            .find(|c| MAIN.contains(&c.as_str()))
            .cloned()
    }
}

/// Directories searched in priority order (highest first).
pub fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let home = std::env::var("HOME").unwrap_or_default();
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(&home).join(".local/share"));
    dirs.push(data_home.join("applications"));
    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for d in data_dirs.split(':').filter(|d| !d.is_empty()) {
        dirs.push(PathBuf::from(d).join("applications"));
    }
    // Flatpak exports are usually in XDG_DATA_DIRS already; add them defensively.
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    dirs.push(data_home.join("flatpak/exports/share/applications"));
    dirs.dedup();
    dirs
}

/// Scan every application directory. Entries with the same desktop id in a
/// higher-priority directory shadow lower ones, per the spec.
pub fn scan_applications() -> Vec<AppEntry> {
    let current_desktop: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    let mut by_id: HashMap<String, AppEntry> = HashMap::new();
    let mut hidden_ids: Vec<String> = Vec::new();
    for dir in application_dirs() {
        let mut files = Vec::new();
        collect_desktop_files(&dir, &dir, &mut files);
        for (id, path) in files {
            if by_id.contains_key(&id) || hidden_ids.contains(&id) {
                continue;
            }
            match parse_desktop_file(&path, &id, &current_desktop) {
                Parsed::Entry(app) => {
                    by_id.insert(id, app);
                }
                Parsed::Hidden => hidden_ids.push(id),
                Parsed::Skip => {}
            }
        }
    }
    let mut apps: Vec<AppEntry> = by_id.into_values().collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn collect_desktop_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_desktop_files(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "desktop") {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let id = rel.with_extension("").to_string_lossy().replace('/', "-");
            out.push((id, path));
        }
    }
}

#[allow(clippy::large_enum_variant)]
enum Parsed {
    Entry(AppEntry),
    /// `Hidden=true` shadows lower-priority entries with the same id.
    Hidden,
    Skip,
}

fn parse_bool(v: &str) -> bool {
    v.trim().eq_ignore_ascii_case("true")
}

fn parse_list(v: &str) -> Vec<String> {
    v.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn parse_desktop_file(path: &Path, id: &str, current_desktop: &[String]) -> Parsed {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Parsed::Skip;
    };
    let mut fields: HashMap<String, String> = HashMap::new();
    let mut in_entry = false;
    let lang = std::env::var("LANG").unwrap_or_default();
    let lang_short: String = lang.split(['_', '.']).next().unwrap_or("").to_string();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim();
        // Localised keys: prefer exact locale, then language only.
        if let Some((base, loc)) = k.split_once('[') {
            let loc = loc.trim_end_matches(']');
            let prio = if !lang.is_empty() && lang.starts_with(loc) {
                2
            } else if loc == lang_short {
                1
            } else {
                0
            };
            if prio > 0 {
                let key = format!("{base}\u{0}{prio}");
                fields.insert(key, v.to_string());
            }
            continue;
        }
        fields.insert(k.to_string(), v.to_string());
    }
    let get = |key: &str| -> Option<String> {
        fields
            .get(&format!("{key}\u{0}2"))
            .or_else(|| fields.get(&format!("{key}\u{0}1")))
            .or_else(|| fields.get(key))
            .cloned()
    };
    if fields.get("Type").map(String::as_str) != Some("Application") {
        return Parsed::Skip;
    }
    if fields.get("Hidden").is_some_and(|v| parse_bool(v)) {
        return Parsed::Hidden;
    }
    if fields.get("NoDisplay").is_some_and(|v| parse_bool(v)) {
        return Parsed::Skip;
    }
    if let Some(only) = fields.get("OnlyShowIn") {
        let list = parse_list(only);
        if !list
            .iter()
            .any(|d| current_desktop.iter().any(|c| c.eq_ignore_ascii_case(d)))
        {
            return Parsed::Skip;
        }
    }
    if let Some(not) = fields.get("NotShowIn") {
        let list = parse_list(not);
        if list
            .iter()
            .any(|d| current_desktop.iter().any(|c| c.eq_ignore_ascii_case(d)))
        {
            return Parsed::Skip;
        }
    }
    if let Some(try_exec) = fields.get("TryExec") {
        if !executable_exists(try_exec) {
            return Parsed::Skip;
        }
    }
    let Some(exec) = fields.get("Exec").cloned() else {
        return Parsed::Skip;
    };
    let Some(name) = get("Name") else {
        return Parsed::Skip;
    };
    Parsed::Entry(AppEntry {
        id: id.to_string(),
        name,
        generic_name: get("GenericName"),
        exec,
        icon: fields.get("Icon").cloned(),
        categories: fields
            .get("Categories")
            .map(|c| parse_list(c))
            .unwrap_or_default(),
        keywords: get("Keywords").map(|k| parse_list(&k)).unwrap_or_default(),
        comment: get("Comment"),
        terminal: fields.get("Terminal").is_some_and(|v| parse_bool(v)),
        path: fields.get("Path").cloned().filter(|p| !p.is_empty()),
        startup_wm_class: fields.get("StartupWMClass").cloned(),
        file: path.to_path_buf(),
    })
}

pub fn executable_exists(name: &str) -> bool {
    let p = Path::new(name);
    if p.is_absolute() {
        return p.exists();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

/// Expand the `Exec=` line into a shell-ready command: strips field codes, expands `%c`
/// (name) and `%k` (file path), and unescapes per the spec.
pub fn expand_exec(app: &AppEntry) -> String {
    let mut out = String::new();
    let mut chars = app.exec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('%') => out.push('%'),
                Some('c') => out.push_str(&shell_quote(&app.name)),
                Some('k') => out.push_str(&shell_quote(&app.file.to_string_lossy())),
                // %f %F %u %U %d %D %n %N %i %v %m: no argument to substitute when launched from a menu.
                Some(_) => {}
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_and_respects_hidden_and_onlyshowin() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.desktop"), "[Desktop Entry]\nType=Application\nName=Alpha\nName[de]=Alfa\nExec=alpha %U --flag\nCategories=Utility;\nKeywords=one;two;\nTerminal=true\n").unwrap();
        std::fs::write(
            root.join("b.desktop"),
            "[Desktop Entry]\nType=Application\nName=Beta\nExec=beta\nOnlyShowIn=GNOME;\n",
        )
        .unwrap();
        std::fs::write(
            root.join("c.desktop"),
            "[Desktop Entry]\nType=Application\nName=Gamma\nExec=gamma\nHidden=true\n",
        )
        .unwrap();
        std::fs::write(root.join("d.desktop"), "[Desktop Entry]\nType=Application\nName=Delta\nExec=delta\nTryExec=/definitely/not/here\n").unwrap();
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(
            root.join("sub/e.desktop"),
            "[Desktop Entry]\nType=Application\nName=Epsilon\nExec=eps %c\n",
        )
        .unwrap();
        let mut files = Vec::new();
        collect_desktop_files(root, root, &mut files);
        let desktop = vec!["eDEX-DE".to_string()];
        let parsed: Vec<(String, Parsed)> = files
            .into_iter()
            .map(|(id, p)| (id.clone(), parse_desktop_file(&p, &id, &desktop)))
            .collect();
        let entries: Vec<&AppEntry> = parsed
            .iter()
            .filter_map(|(_, p)| {
                if let Parsed::Entry(e) = p {
                    Some(e)
                } else {
                    None
                }
            })
            .collect();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"Epsilon"));
        assert!(!names.contains(&"Beta"));
        assert!(!names.contains(&"Delta"));
        assert!(parsed
            .iter()
            .any(|(id, p)| id == "c" && matches!(p, Parsed::Hidden)));
        assert!(parsed.iter().any(|(id, _)| id == "sub-e"));
        let alpha = entries.iter().find(|e| e.name == "Alpha").unwrap();
        assert!(alpha.terminal);
        assert_eq!(alpha.keywords, vec!["one", "two"]);
        assert_eq!(expand_exec(alpha), "alpha --flag");
        let eps = entries.iter().find(|e| e.name == "Epsilon").unwrap();
        assert_eq!(expand_exec(eps), "eps 'Epsilon'");
    }
}
