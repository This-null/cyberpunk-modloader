use crate::library::ModRecord;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Conflict {
    pub file: String,
    pub owner: Option<String>,
}

pub fn find_conflicts(game: &Path, records: &[ModRecord], candidate: &ModRecord) -> Vec<Conflict> {
    let mut out = Vec::new();
    for file in &candidate.files {
        let owner = records
            .iter()
            .find(|r| r.name != candidate.name && r.enabled && r.installed_files.iter().any(|f| f.eq_ignore_ascii_case(file)));
        if let Some(o) = owner {
            out.push(Conflict {
                file: file.clone(),
                owner: Some(o.name.clone()),
            });
        } else if game.join(file).exists() && !candidate.installed_files.iter().any(|f| f.eq_ignore_ascii_case(file)) {
            out.push(Conflict {
                file: file.clone(),
                owner: None,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, enabled: bool, files: &[&str], installed: &[&str]) -> ModRecord {
        ModRecord {
            name: name.into(),
            enabled,
            files: files.iter().map(|s| s.to_string()).collect(),
            installed_files: installed.iter().map(|s| s.to_string()).collect(),
            created_dirs: Vec::new(),
            ..ModRecord::default()
        }
    }

    #[test]
    fn detects_other_mod_owner() {
        let dir = tempfile::tempdir().unwrap();
        let a = record("a", true, &["x.archive"], &["x.archive"]);
        let b = record("b", false, &["x.archive"], &[]);
        let found = find_conflicts(dir.path(), &[a.clone(), b.clone()], &b);
        assert_eq!(
            found,
            vec![Conflict {
                file: "x.archive".into(),
                owner: Some("a".into())
            }]
        );
    }

    #[test]
    fn detects_untracked_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.archive"), "old").unwrap();
        let b = record("b", false, &["x.archive"], &[]);
        let found = find_conflicts(dir.path(), &[b.clone()], &b);
        assert_eq!(
            found,
            vec![Conflict {
                file: "x.archive".into(),
                owner: None
            }]
        );
    }

    #[test]
    fn paths_differing_only_in_case_are_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = record("a", true, &["Archive/pc/mod/x.archive"], &["Archive/pc/mod/x.archive"]);
        let b = record("b", false, &["archive/PC/mod/X.archive"], &[]);
        let found = find_conflicts(dir.path(), &[a, b.clone()], &b);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].owner.as_deref(), Some("a"));
    }

    #[test]
    fn ignores_own_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.archive"), "mine").unwrap();
        let b = record("b", true, &["x.archive"], &["x.archive"]);
        assert!(find_conflicts(dir.path(), &[b.clone()], &b).is_empty());
    }

    #[test]
    fn disabled_mods_do_not_own_files() {
        let dir = tempfile::tempdir().unwrap();
        let a = record("a", false, &["x.archive"], &[]);
        let b = record("b", false, &["x.archive"], &[]);
        assert!(find_conflicts(dir.path(), &[a, b.clone()], &b).is_empty());
    }
}
