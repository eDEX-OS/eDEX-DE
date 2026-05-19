use std::{
    fs,
    path::{Component, PathBuf},
    process::Command,
};

#[allow(dead_code)]
pub struct FilesystemPanel {
    pub cwd: PathBuf,
    pub entries: Vec<FsEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub show_dotfiles: bool,
    pub max_visible: usize,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub extension: Option<String>,
    pub is_hidden: bool,
    pub is_symlink: bool,
}

impl Default for FilesystemPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesystemPanel {
    pub fn new() -> Self {
        let cwd = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/"));
        let mut panel = Self {
            cwd,
            entries: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            show_dotfiles: false,
            max_visible: 30,
        };
        panel.refresh();
        panel
    }

    pub fn refresh(&mut self) {
        let mut entries = fs::read_dir(&self.cwd)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.flatten())
            .filter_map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_hidden = name.starts_with('.');
                if is_hidden && !self.show_dotfiles {
                    return None;
                }

                let symlink_meta = fs::symlink_metadata(&path).ok()?;
                let metadata = fs::metadata(&path).ok();
                let is_dir = metadata.as_ref().is_some_and(|meta| meta.is_dir());
                let is_symlink = symlink_meta.file_type().is_symlink();
                let size = metadata
                    .as_ref()
                    .filter(|meta| meta.is_file())
                    .map(std::fs::Metadata::len);
                let extension = path
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string());

                Some(FsEntry {
                    name,
                    path,
                    is_dir,
                    size,
                    extension,
                    is_hidden,
                    is_symlink,
                })
            })
            .collect::<Vec<_>>();

        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        self.entries = entries;
        if self.entries.is_empty() {
            self.selected = 0;
            self.scroll_offset = 0;
            return;
        }

        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
        self.clamp_scroll();
    }

    pub fn navigate_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.clamp_scroll();
        }
    }

    pub fn navigate_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            self.clamp_scroll();
        }
    }

    pub fn enter_selected(&mut self) {
        let Some(entry) = self.entries.get(self.selected).cloned() else {
            return;
        };

        if entry.is_dir {
            self.cwd = entry.path;
            self.selected = 0;
            self.scroll_offset = 0;
            self.refresh();
        } else {
            let _ = Command::new("xdg-open").arg(&entry.path).spawn();
        }
    }

    pub fn go_parent(&mut self) {
        if let Some(parent) = self.cwd.parent().map(PathBuf::from) {
            self.cwd = parent;
            self.refresh();
        }
    }

    pub fn toggle_dotfiles(&mut self) {
        self.show_dotfiles = !self.show_dotfiles;
        self.selected = 0;
        self.scroll_offset = 0;
        self.refresh();
    }

    pub fn set_panel_height(&mut self, panel_height: u32) {
        self.max_visible = ((panel_height.saturating_sub(32)) / 22).max(1) as usize;
        self.clamp_scroll();
    }

    pub fn visible_entries(&self) -> &[FsEntry] {
        let end = (self.scroll_offset + self.max_visible).min(self.entries.len());
        &self.entries[self.scroll_offset..end]
    }

    pub fn breadcrumbs(&self) -> Vec<String> {
        let mut crumbs = Vec::new();
        for component in self.cwd.components() {
            match component {
                Component::RootDir => crumbs.push("/".to_string()),
                Component::Normal(part) => crumbs.push(part.to_string_lossy().to_string()),
                Component::Prefix(prefix) => {
                    crumbs.push(prefix.as_os_str().to_string_lossy().to_string())
                }
                _ => {}
            }
        }
        if crumbs.is_empty() {
            crumbs.push("/".to_string());
        }
        crumbs
    }

    pub fn selected_visible_index(&self) -> usize {
        self.selected.saturating_sub(self.scroll_offset)
    }

    pub fn to_ui_entries(&self) -> Vec<super::state::FsEntry> {
        self.visible_entries()
            .iter()
            .map(|entry| super::state::FsEntry {
                name: format!("{} {}", icon_for_entry(entry), entry.name),
                is_dir: entry.is_dir,
            })
            .collect()
    }

    fn clamp_scroll(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
        if self.selected >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected + 1 - self.max_visible;
        }
    }
}

fn icon_for_entry(entry: &FsEntry) -> &'static str {
    if entry.is_dir {
        "📁"
    } else if entry.is_symlink {
        "🔗"
    } else {
        match entry
            .extension
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("rs") => "🦀",
            Some("toml") | Some("yaml") | Some("yml") | Some("json") => "⚙️",
            Some("md") | Some("txt") => "📄",
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") => "🖼️",
            Some("zip") | Some("tar") | Some("gz") | Some("xz") => "📦",
            Some("sh") => "💻",
            _ => "📄",
        }
    }
}
