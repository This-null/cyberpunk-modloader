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

const WRAPPED_ROOTS: [(&str, &[&str]); 5] = [
    ("archive", &["pc"]),
    ("bin", &["x64"]),
    ("r6", &["scripts", "tweaks", "config", "cache", "inputs", "audioware"]),
    ("red4ext", &["plugins"]),
    ("engine", &["tools", "config"]),
];

fn is_root_name(seg: &str) -> bool {
    ROOTS.contains(&seg.to_ascii_lowercase().as_str())
}

fn wrapper_start(segments: &[&str]) -> usize {
    if segments.first().is_some_and(|s| is_root_name(s)) {
        return 0;
    }
    for i in 1..segments.len().min(4) {
        let here = segments[i].to_ascii_lowercase();
        let Some((_, nexts)) = WRAPPED_ROOTS.iter().find(|(r, _)| *r == here) else { continue };
        if segments.get(i + 1).is_some_and(|n| nexts.contains(&n.to_ascii_lowercase().as_str())) {
            return i;
        }
    }
    0
}

pub fn is_rooted(raw: &str) -> bool {
    let norm = raw.replace('\\', "/");
    let segments: Vec<&str> = norm.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    segments.first().is_some_and(|_| is_root_name(segments[wrapper_start(&segments)]))
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
    if segments.is_empty() {
        return Ok(None);
    }
    let checked = segments.clone();
    let segments: Vec<&str> = segments[wrapper_start(&segments)..].to_vec();
    let first = segments[0];
    let tool = segments.len() >= 3
        && segments[0].eq_ignore_ascii_case("engine")
        && segments[1].eq_ignore_ascii_case("tools");
    if checked.iter().any(|s| bad_segment(s))
        || (!tool && segments.last().is_some_and(|s| s.to_ascii_lowercase().ends_with(".exe")))
    {
        return Err(AppError::UnsafePath(raw.to_string()));
    }
    if ROOTS.contains(&first.to_ascii_lowercase().as_str()) {
        return Ok(Some(segments.iter().collect()));
    }
    let name = segments.last().copied().unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    if [".md", ".txt", ".pdf", ".doc", ".docx", ".rtf", ".odt", ".html", ".htm", ".nfo"].iter().any(|e| lower.ends_with(e)) || crate::meta::is_image(&lower) {
        return Ok(None);
    }
    if [".url", ".lnk"].iter().any(|e| lower.ends_with(e)) {
        return Ok(None);
    }
    if [".bat", ".cmd", ".ps1", ".vbs", ".sh"].iter().any(|e| lower.ends_with(e)) {
        return Ok(Some(segments.iter().collect()));
    }
    if segments.len() == 1 && lower.starts_with("launcher-") {
        return Ok(None);
    }
    if lower.ends_with(".archive") || lower.ends_with(".xl") {
        return Ok(Some(PathBuf::from("archive/pc/mod").join(segments.iter().collect::<PathBuf>())));
    }
    if lower.ends_with(".reds") {
        let under = if segments.len() == 1 { PathBuf::from(mod_name).join(name) } else { segments.iter().collect() };
        return Ok(Some(PathBuf::from("r6/scripts").join(under)));
    }
    Err(AppError::Unmappable(raw.to_string()))
}

fn cet_dirs(rels: &[&str]) -> Vec<Vec<String>> {
    let mut dirs: Vec<Vec<String>> = rels
        .iter()
        .filter_map(|r| {
            let norm = r.replace('\\', "/");
            let segs: Vec<String> = norm.split('/').filter(|s| !s.is_empty() && *s != ".").map(str::to_string).collect();
            let last = segs.last()?;
            (last.eq_ignore_ascii_case("init.lua") && !is_root_name(&segs[0])).then(|| segs[..segs.len() - 1].to_vec())
        })
        .collect();
    dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
    dirs
}

fn map_extra(raw: &str, mod_name: &str, cet: &[Vec<String>]) -> Option<PathBuf> {
    let norm = raw.replace('\\', "/");
    let segs: Vec<&str> = norm.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    let name = segs.last()?;
    let lower = name.to_ascii_lowercase();
    for dir in cet {
        let inside = segs.len() > dir.len() && dir.iter().zip(&segs).all(|(a, b)| a.eq_ignore_ascii_case(b));
        if inside {
            let folder = dir.last().map(String::as_str).unwrap_or(mod_name);
            let rest: PathBuf = segs[dir.len()..].iter().collect();
            return Some(PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods").join(folder).join(rest));
        }
    }
    if [".tweak", ".yaml", ".yml"].iter().any(|e| lower.ends_with(e)) {
        return Some(PathBuf::from("r6/tweaks").join(segs.iter().collect::<PathBuf>()));
    }
    if lower.ends_with(".dll") {
        let under = if segs.len() == 1 { PathBuf::from(mod_name).join(name) } else { segs.iter().collect() };
        return Some(PathBuf::from("red4ext/plugins").join(under));
    }
    None
}

fn is_shortcut(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.ends_with(".url") || lower.ends_with(".lnk")
}

pub fn map_all(rels: &[&str], mod_name: &str) -> Result<(Vec<Option<PathBuf>>, Vec<String>), AppError> {
    if rels.iter().any(|r| r.replace('\\', "/").to_ascii_lowercase().ends_with("fomod/moduleconfig.xml")) {
        return Err(AppError::Fomod);
    }
    let cet = cet_dirs(rels);
    let mut ignored = Vec::new();
    let mut out = Vec::with_capacity(rels.len());
    for r in rels {
        match map_entry(r, mod_name) {
            Err(AppError::Unmappable(_)) => match map_extra(r, mod_name, &cet) {
                Some(p) => out.push(Some(p)),
                None => {
                    ignored.push(r.to_string());
                    out.push(None);
                }
            },
            Ok(None) if !r.replace('\\', "/").ends_with('/') && !is_shortcut(r) => match map_extra(r, mod_name, &cet) {
                Some(p) if p.starts_with("bin/x64/plugins/cyber_engine_tweaks") => out.push(Some(p)),
                _ => out.push(None),
            },
            other => out.push(other?),
        }
    }
    Ok((out, ignored))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn helper_scripts_keep_their_place_in_the_game_folder_and_shortcuts_are_dropped() {
        assert_eq!(map_entry("Cyberclean.bat", "m").unwrap(), Some(PathBuf::from("Cyberclean.bat")));
        assert_eq!(map_entry("Pack/run.cmd", "m").unwrap(), Some(PathBuf::from("Pack/run.cmd")));
        assert_eq!(map_entry("bin/x64/tool.bat", "m").unwrap(), Some(PathBuf::from("bin/x64/tool.bat")));
        for f in ["site.url", "link.lnk"] {
            assert_eq!(map_entry(f, "m").unwrap(), None, "{f}");
        }
    }

    #[test]
    fn readme_style_documents_of_any_common_kind_are_skipped() {
        for f in ["Hex Editing Colors and Surfaces in Cyberpunk 2077.pdf", "guide.docx", "Docs/manual.rtf", "info.nfo", "index.html"] {
            assert_eq!(map_entry(f, "m").unwrap(), None, "{f}");
        }
    }

    #[test]
    fn a_wrapper_folder_around_a_game_tree_is_stripped() {
        assert_eq!(map_entry("My Mod 1.0/archive/pc/mod/x.archive", "m").unwrap(), Some(PathBuf::from("archive/pc/mod/x.archive")));
        assert_eq!(
            map_entry("Pack\\bin\\x64\\plugins\\cyber_engine_tweaks\\mods\\a\\init.lua", "m").unwrap(),
            Some(PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods/a/init.lua"))
        );
        assert_eq!(map_entry("Mod/r6/scripts/a/b.reds", "m").unwrap(), Some(PathBuf::from("r6/scripts/a/b.reds")));
        assert_eq!(map_entry("Pack/archive/readme.md", "m").unwrap(), None);
        assert_eq!(map_entry("Pack/archive/stuff.archive", "m").unwrap(), Some(PathBuf::from("archive/pc/mod/Pack/archive/stuff.archive")));
        assert!(is_rooted("Wrapper/archive/pc/mod/a.archive") && !is_rooted("Wrapper/archive/a.archive"));
    }

    #[test]
    fn cet_tweak_and_plugin_files_find_their_place() {
        let rels = ["CoolMod/init.lua", "CoolMod/ui/panel.lua", "CoolMod/data.json", "Other/a.tweak", "n.yaml", "plugin.dll", "Sub/b.dll"];
        let out = map_all(&rels, "mm").unwrap().0;
        let cet = "bin/x64/plugins/cyber_engine_tweaks/mods/CoolMod";
        assert_eq!(out[0], Some(PathBuf::from(format!("{cet}/init.lua"))));
        assert_eq!(out[1], Some(PathBuf::from(format!("{cet}/ui/panel.lua"))));
        assert_eq!(out[2], Some(PathBuf::from(format!("{cet}/data.json"))));
        assert_eq!(out[3], Some(PathBuf::from("r6/tweaks/Other/a.tweak")));
        assert_eq!(out[4], Some(PathBuf::from("r6/tweaks/n.yaml")));
        assert_eq!(out[5], Some(PathBuf::from("red4ext/plugins/mm/plugin.dll")));
        assert_eq!(out[6], Some(PathBuf::from("red4ext/plugins/Sub/b.dll")));
    }

    #[test]
    fn a_cet_mod_with_init_lua_at_the_archive_root_goes_under_the_mod_name() {
        let out = map_all(&["init.lua", "lib/x.lua"], "Fancy").unwrap().0;
        assert_eq!(out[0], Some(PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods/Fancy/init.lua")));
        assert_eq!(out[1], Some(PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods/Fancy/lib/x.lua")));
    }

    #[test]
    fn fomod_installers_are_refused_and_unknown_files_still_fail() {
        assert!(matches!(map_all(&["fomod/ModuleConfig.xml", "a/archive/pc/mod/x.archive"], "m"), Err(AppError::Fomod)));
        let (out, ignored) = map_all(&["thing.bin", "a.archive"], "m").unwrap();
        assert_eq!(out, vec![None, Some(PathBuf::from("archive/pc/mod/a.archive"))]);
        assert_eq!(ignored, vec!["thing.bin".to_string()]);
        assert!(matches!(map_all(&["../x.archive"], "m"), Err(AppError::UnsafePath(_))));
    }

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
    fn files_inside_a_wrapper_folder_keep_the_folder_under_the_game_folder() {
        assert_eq!(
            map_entry("MidLength_HAIRS_WINGDEER/#VESS_HAIR_NO32.archive", "m").unwrap().unwrap(),
            PathBuf::from("archive/pc/mod/MidLength_HAIRS_WINGDEER/#VESS_HAIR_NO32.archive")
        );
        assert_eq!(
            map_entry("Pack\\Sub\\b.archive.xl", "m").unwrap().unwrap(),
            PathBuf::from("archive/pc/mod/Pack/Sub/b.archive.xl")
        );
        assert_eq!(map_entry("Pack/x.reds", "m").unwrap().unwrap(), PathBuf::from("r6/scripts/Pack/x.reds"));
        assert!(matches!(map_entry("Pack/tool.bin", "m"), Err(AppError::Unmappable(_))));
        assert!(matches!(map_entry("Pack/../x.archive", "m"), Err(AppError::UnsafePath(_))));
        assert!(map_entry("Pack/readme.txt", "m").unwrap().is_none());
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
        assert_eq!(
            map_entry("engine/tools/scc.exe", "m").unwrap().unwrap(),
            PathBuf::from("engine/tools/scc.exe")
        );
        assert_eq!(
            map_entry("ENGINE\\Tools\\scc.EXE", "m").unwrap().unwrap(),
            PathBuf::from("ENGINE/Tools/scc.EXE")
        );
        for bad in ["engine/scc.exe", "engine/tools.exe", "bin/x64/tools/scc.exe", "r6/scripts/scc.exe"] {
            assert!(matches!(map_entry(bad, "m"), Err(AppError::UnsafePath(_))), "{bad}");
        }
        assert!(map_entry("archive/pc/mod/console.archive", "m").unwrap().is_some());
        assert!(map_entry("archive/pc/mod/com10.archive", "m").unwrap().is_some());
    }

    #[test]
    fn text_files_inside_a_cet_mod_folder_go_with_the_mod_but_stray_documents_stay_out() {
        let (out, ignored) = map_all(&["Cool/init.lua", "Cool/lang.txt", "Cool/readme.md", "notes.txt", "Cool/"], "m").unwrap();
        assert_eq!(out[0], Some(PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods/Cool/init.lua")));
        assert_eq!(out[1], Some(PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods/Cool/lang.txt")));
        assert_eq!(out[2], Some(PathBuf::from("bin/x64/plugins/cyber_engine_tweaks/mods/Cool/readme.md")));
        assert_eq!((out[3].clone(), out[4].clone(), ignored.len()), (None, None, 0));
    }

    #[test]
    fn docs_are_skipped_not_rejected() {
        assert!(map_entry("README.md", "m").unwrap().is_none());
        assert!(map_entry("license.txt", "m").unwrap().is_none());
        assert!(map_entry("Wrapper/notes.txt", "m").unwrap().is_none());
    }
}
