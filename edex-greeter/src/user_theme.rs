//! Per-user login theme. eDEX-DE writes the user's theme name to
//! `/var/lib/edex-greeter/themes/<user>` (a sticky, world-writable directory) whenever it
//! changes, so the login screen shows each user in their own colours.

use std::{os::unix::fs::MetadataExt, path::Path};

pub const DIR: &str = "/var/lib/edex-greeter/themes";

/// The theme name `user` (uid `uid`) published, if the file is a small regular file that
/// really belongs to them (anyone can create files in the directory).
pub fn read(dir: &Path, user: &str, uid: u32) -> Option<String> {
    if user.is_empty() || user.contains('/') || user.starts_with('.') {
        return None;
    }
    let path = dir.join(user);
    let meta = std::fs::symlink_metadata(&path).ok()?;
    if !meta.file_type().is_file() || meta.uid() != uid || meta.len() > 128 {
        return None;
    }
    let name = std::fs::read_to_string(&path).ok()?;
    let name = name.trim();
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_files_owned_by_the_user() {
        let dir = tempfile::tempdir().unwrap();
        let me = std::fs::metadata(dir.path()).unwrap().uid();
        std::fs::write(dir.path().join("ari"), "matrix\n").unwrap();
        assert_eq!(read(dir.path(), "ari", me).as_deref(), Some("matrix"));
        assert_eq!(read(dir.path(), "ari", me + 1), None);
        std::fs::write(dir.path().join("bad"), "../../etc").unwrap();
        assert_eq!(read(dir.path(), "bad", me), None);
        std::os::unix::fs::symlink(dir.path().join("ari"), dir.path().join("link")).unwrap();
        assert_eq!(read(dir.path(), "link", me), None);
        assert_eq!(read(dir.path(), "../ari", me), None);
    }
}
