use super::colors::Theme;

#[derive(Clone, Debug)]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug)]
pub struct UiState {
    pub clock: String,
    pub hostname: String,
    pub theme: &'static Theme,
    pub terminal_content: Vec<String>,
    pub filesystem_cwd: String,
    pub filesystem_entries: Vec<FsEntry>,
}
