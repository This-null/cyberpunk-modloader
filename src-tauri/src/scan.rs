use crate::library::ModRecord;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ExternalMod {
    pub name: String,
    pub kind: String,
    pub files: Vec<String>,
}

const SUFFIXES: [&str; 3] = [".archive.xl", ".archive", ".xl"];

fn stem(file: &str) -> String {
    let lower = file.to_ascii_lowercase();
    for suffix in SUFFIXES {
        if lower.ends_with(suffix) {
            return file[..file.len() - suffix.len()].to_string();
        }
    }
    match file.rsplit_once('.') {
        Some((base, _)) if !base.is_empty() => base.to_string(),
        _ => file.to_string(),
    }
}

const MAX_DEPTH: usize = 12;

fn is_log(path: &Path) -> bool {
    path.extension().is_some_and(|x| x.eq_ignore_ascii_case("log"))
}

fn walk(dir: &Path, game: &Path, out: &mut Vec<String>, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, game, out, depth + 1);
        } else if is_log(&path) {
        } else if let Ok(rel) = path.strip_prefix(game) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn collect(game: &Path, rel: &str, kind: &str, loose: bool, out: &mut Vec<ExternalMod>) {
    let Ok(entries) = fs::read_dir(game.join(rel)) else {
        return;
    };
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            let mut files = Vec::new();
            walk(&path, game, &mut files, 0);
            if !files.is_empty() {
                grouped.entry(name).or_default().extend(files);
            }
        } else if loose && !is_log(&path) {
            if let Ok(rel_path) = path.strip_prefix(game) {
                grouped
                    .entry(stem(&name))
                    .or_default()
                    .push(rel_path.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    for (name, mut files) in grouped {
        files.sort();
        out.push(ExternalMod {
            name,
            kind: kind.to_string(),
            files,
        });
    }
}

pub fn scan_external(game: &Path, records: &[ModRecord]) -> Vec<ExternalMod> {
    let mut found = Vec::new();
    collect(game, "archive/pc/mod", "archive", true, &mut found);
    collect(game, "bin/x64/plugins/cyber_engine_tweaks/mods", "cet", false, &mut found);
    collect(game, "r6/scripts", "redscript", true, &mut found);
    collect(game, "r6/tweaks", "tweaks", true, &mut found);
    collect(game, "red4ext/plugins", "red4ext", false, &mut found);

    let owned: HashSet<String> = records
        .iter()
        .filter(|r| r.enabled)
        .flat_map(|r| r.installed_files.iter())
        .map(|f| f.to_ascii_lowercase())
        .collect();
    found.retain(|m| !m.files.iter().any(|f| owned.contains(&f.to_ascii_lowercase())));

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for m in &found {
        *counts.entry(m.name.clone()).or_default() += 1;
    }
    for m in &mut found {
        if counts[&m.name] > 1 {
            m.name = format!("{} ({})", m.name, m.kind);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(game: &Path, rel: &str) {
        let p = game.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "x").unwrap();
    }

    fn names(list: &[ExternalMod]) -> Vec<(String, String)> {
        list.iter().map(|m| (m.name.clone(), m.kind.clone())).collect()
    }

    fn sample() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let g = tmp.path();
        put(g, "archive/pc/mod/foo.archive");
        put(g, "archive/pc/mod/foo.archive.xl");
        put(g, "archive/pc/mod/bar.archive");
        put(g, "bin/x64/plugins/cyber_engine_tweaks/mods/Cool/init.lua");
        put(g, "bin/x64/plugins/cyber_engine_tweaks/mods/Cool/data/x.json");
        put(g, "red4ext/plugins/NightCoop/NightCoop.dll");
        put(g, "r6/scripts/loose.reds");
        fs::create_dir_all(g.join("r6/scripts/empty")).unwrap();
        tmp
    }

    #[test]
    fn plugin_log_files_are_not_mods() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), "red4ext/plugins/NightCoop/logs/nightcoop.log");
        put(tmp.path(), "red4ext/plugins/NightCoop/crash.LOG");
        assert!(scan_external(tmp.path(), &[]).is_empty());
        put(tmp.path(), "red4ext/plugins/NightCoop/NightCoop.dll");
        let found = scan_external(tmp.path(), &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].files, vec!["red4ext/plugins/NightCoop/NightCoop.dll".to_string()]);
    }

    #[test]
    fn finds_mods_of_every_kind() {
        let tmp = sample();
        let found = scan_external(tmp.path(), &[]);
        assert_eq!(
            names(&found),
            vec![
                ("bar".into(), "archive".into()),
                ("foo".into(), "archive".into()),
                ("Cool".into(), "cet".into()),
                ("loose".into(), "redscript".into()),
                ("NightCoop".into(), "red4ext".into()),
            ]
        );
    }

    #[test]
    fn groups_archive_and_xl_under_one_name() {
        let tmp = sample();
        let found = scan_external(tmp.path(), &[]);
        let foo = found.iter().find(|m| m.name == "foo").unwrap();
        assert_eq!(
            foo.files,
            vec!["archive/pc/mod/foo.archive", "archive/pc/mod/foo.archive.xl"]
        );
    }

    #[test]
    fn walks_nested_folders_and_skips_empty_ones() {
        let tmp = sample();
        let found = scan_external(tmp.path(), &[]);
        let cool = found.iter().find(|m| m.name == "Cool").unwrap();
        assert_eq!(cool.files.len(), 2);
        assert!(!found.iter().any(|m| m.name == "empty"));
    }

    #[test]
    fn hides_mods_the_launcher_already_owns() {
        let tmp = sample();
        let record = ModRecord {
            name: "foo".into(),
            enabled: true,
            files: vec![],
            installed_files: vec!["archive/pc/mod/foo.archive".into()],
            created_dirs: vec![],
            ..ModRecord::default()
        };
        let found = scan_external(tmp.path(), &[record]);
        assert!(!found.iter().any(|m| m.name == "foo"));
        assert!(found.iter().any(|m| m.name == "bar"));
    }

    #[test]
    fn disambiguates_equal_names_across_kinds() {
        let tmp = tempfile::tempdir().unwrap();
        put(tmp.path(), "archive/pc/mod/same.archive");
        put(tmp.path(), "red4ext/plugins/same/a.dll");
        let found = scan_external(tmp.path(), &[]);
        let mut n: Vec<String> = found.iter().map(|m| m.name.clone()).collect();
        n.sort();
        assert_eq!(n, vec!["same (archive)", "same (red4ext)"]);
    }

    #[test]
    fn very_deep_folders_do_not_run_the_scan_away() {
        let tmp = tempfile::tempdir().unwrap();
        let mut rel = String::from("red4ext/plugins/deep");
        for i in 0..40 {
            rel.push_str(&format!("/d{i}"));
        }
        put(tmp.path(), &format!("{rel}/x.dll"));
        let found = scan_external(tmp.path(), &[]);
        assert!(found.iter().all(|m| m.name != "deep" || m.files.is_empty()));
    }

    #[test]
    fn ownership_ignores_letter_case() {
        let tmp = sample();
        let record = ModRecord {
            name: "foo".into(),
            enabled: true,
            installed_files: vec!["Archive/PC/mod/FOO.archive".into()],
            ..ModRecord::default()
        };
        let found = scan_external(tmp.path(), &[record]);
        assert!(!found.iter().any(|m| m.name == "foo"));
    }

    #[test]
    fn missing_game_folders_give_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(scan_external(tmp.path(), &[]).is_empty());
    }
}
