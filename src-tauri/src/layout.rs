use crate::error::AppError;
use std::path::PathBuf;

const ROOTS: [&str; 6] = ["archive", "bin", "r6", "red4ext", "engine", "mods"];

pub fn is_reserved_device(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    let bytes = stem.as_bytes();
    bytes.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && (b'1'..=b'9').contains(&bytes[3])
}

pub fn bad_segment(seg: &str) -> bool {
    seg.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c))
        || seg.ends_with('.')
        || seg.ends_with(' ')
        || is_reserved_device(seg)
}

pub fn is_rooted(raw: &str) -> bool {
    raw.replace('\\', "/")
        .split('/')
        .find(|s| !s.is_empty() && *s != ".")
        .is_some_and(|first| ROOTS.contains(&first.to_ascii_lowercase().as_str()))
}

pub fn map_entry(raw: &str, mod_name: &str) -> Result<Option<PathBuf>, AppError> {
    let norm = raw.replace('\\', "/");
    if norm.is_empty() {
        return Ok(None);
    }
    let bytes = norm.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if norm.starts_with('/') || drive || norm.split('/').any(|s| s == "..") {
        return Err(AppError::UnsafePath(raw.to_string()));
    }
    if norm.ends_with('/') {
        return Ok(None);
    }
    let segments: Vec<&str> = norm
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    let Some(first) = segments.first() else {
        return Ok(None);
    };
    if segments.iter().any(|s| bad_segment(s))
        || segments.last().is_some_and(|s| s.to_ascii_lowercase().ends_with(".exe"))
    {
        return Err(AppError::UnsafePath(raw.to_string()));
    }
    if ROOTS.contains(&first.to_ascii_lowercase().as_str()) {
        return Ok(Some(segments.iter().collect()));
    }
    let name = segments.last().copied().unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".md") || lower.ends_with(".txt") || crate::meta::is_image(&lower) {
        return Ok(None);
    }
    if segments.len() == 1 && lower.starts_with("launcher-") {
        return Ok(None);
    }
    if segments.len() == 1 {
        if lower.ends_with(".archive") || lower.ends_with(".xl") {
            return Ok(Some(PathBuf::from("archive/pc/mod").join(name)));
        }
        if lower.ends_with(".reds") {
            return Ok(Some(PathBuf::from("r6/scripts").join(mod_name).join(name)));
        }
    }
    Err(AppError::Unmappable(raw.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rooted_paths_pass_through() {
        let p = map_entry("archive/pc/mod/a.archive", "m").unwrap().unwrap();
        assert_eq!(p, PathBuf::from("archive/pc/mod/a.archive"));
    }

    #[test]
    fn backslashes_are_normalised() {
        let p = map_entry("bin\\x64\\plugins\\cyber_engine_tweaks\\mods\\x\\init.lua", "m")
            .unwrap()
            .unwrap();
        assert_eq!(
            p,
            PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods/x/init.lua")
        );
    }

    #[test]
    fn flat_archive_goes_to_mod_dir() {
        let p = map_entry("thing.archive", "m").unwrap().unwrap();
        assert_eq!(p, PathBuf::from("archive/pc/mod/thing.archive"));
    }

    #[test]
    fn flat_xl_goes_to_mod_dir() {
        let p = map_entry("thing.archive.xl", "m").unwrap().unwrap();
        assert_eq!(p, PathBuf::from("archive/pc/mod/thing.archive.xl"));
    }

    #[test]
    fn flat_reds_goes_to_named_scripts_dir() {
        let p = map_entry("a.reds", "mymod").unwrap().unwrap();
        assert_eq!(p, PathBuf::from("r6/scripts/mymod/a.reds"));
    }

    #[test]
    fn directory_entries_are_skipped() {
        assert!(map_entry("archive/pc/mod/", "m").unwrap().is_none());
    }

    #[test]
    fn parent_traversal_is_rejected() {
        assert!(matches!(map_entry("../evil.dll", "m"), Err(AppError::UnsafePath(_))));
        assert!(matches!(
            map_entry("archive/../../evil.dll", "m"),
            Err(AppError::UnsafePath(_))
        ));
        assert!(matches!(map_entry("../", "m"), Err(AppError::UnsafePath(_))));
    }

    #[test]
    fn absolute_and_drive_paths_are_rejected() {
        assert!(matches!(map_entry("/etc/x", "m"), Err(AppError::UnsafePath(_))));
        assert!(matches!(map_entry("C:/x.dll", "m"), Err(AppError::UnsafePath(_))));
        assert!(matches!(map_entry("C:\\x.dll", "m"), Err(AppError::UnsafePath(_))));
    }

    #[test]
    fn unknown_flat_file_is_unmappable() {
        assert!(matches!(map_entry("tool.bin", "m"), Err(AppError::Unmappable(_))));
    }

    #[test]
    fn images_and_launcher_files_outside_game_folders_are_skipped() {
        assert!(map_entry("preview.png", "m").unwrap().is_none());
        assert!(map_entry("Wrapper/shot.JPG", "m").unwrap().is_none());
        assert!(map_entry("launcher-mod.json", "m").unwrap().is_none());
        assert!(map_entry("launcher-cover.webp", "m").unwrap().is_none());
    }

    #[test]
    fn images_inside_game_folders_are_kept() {
        let p = map_entry("archive/pc/mod/tex.png", "m").unwrap().unwrap();
        assert_eq!(p, PathBuf::from("archive/pc/mod/tex.png"));
    }

    #[test]
    fn rooted_detection_follows_the_game_folders() {
        assert!(is_rooted("archive/pc/mod/a.archive"));
        assert!(is_rooted("BIN\\x64\\a.dll"));
        assert!(!is_rooted("Wrapper/archive/a.archive"));
        assert!(!is_rooted("preview.png"));
    }

    #[test]
    fn windows_tricks_and_game_executables_are_refused() {
        for bad in [
            "archive/pc/mod/a.archive:hidden",
            "archive/pc/mod/trailing.",
            "archive/pc/mod/trailing ",
            "archive/pc/mod/nul.txt",
            "archive/pc/mod/COM1",
            "bin/x64/Cyberpunk2077.exe",
            "bin/x64/Tool.EXE",
            "r6/scripts/a<b.reds",
        ] {
            assert!(matches!(map_entry(bad, "m"), Err(AppError::UnsafePath(_))), "{bad}");
        }
        assert!(map_entry("bin/x64/winmm.dll", "m").unwrap().is_some());
        assert!(map_entry("archive/pc/mod/console.archive", "m").unwrap().is_some());
        assert!(map_entry("archive/pc/mod/com10.archive", "m").unwrap().is_some());
    }

    #[test]
    fn docs_are_skipped_not_rejected() {
        assert!(map_entry("README.md", "m").unwrap().is_none());
        assert!(map_entry("license.txt", "m").unwrap().is_none());
        assert!(map_entry("Wrapper/notes.txt", "m").unwrap().is_none());
    }
}
