use crate::error::AppError;
use crate::library::ModRecord;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_PRESETS: usize = 30;
const MAX_MODS: usize = 400;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    pub name: String,
    pub mods: Vec<String>,
}

fn file(data: &Path) -> PathBuf {
    data.join("presets.json")
}

pub fn load(data: &Path) -> Vec<Preset> {
    fs::read_to_string(file(data))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(data: &Path, list: &[Preset]) -> Result<(), AppError> {
    fs::create_dir_all(data)?;
    let text = serde_json::to_string_pretty(list).map_err(|e| AppError::State(e.to_string()))?;
    let tmp = file(data).with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, file(data))?;
    Ok(())
}

pub fn clean_name(raw: &str) -> Result<String, AppError> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(|c| c.is_control()) {
        return Err(AppError::InvalidName("preset".into()));
    }
    Ok(name)
}

fn index(list: &[Preset], name: &str) -> Option<usize> {
    list.iter().position(|p| p.name.eq_ignore_ascii_case(name))
}

pub fn upsert(list: &mut Vec<Preset>, name: &str, mods: Vec<String>) -> Result<(), AppError> {
    let name = clean_name(name)?;
    let mut mods = mods;
    mods.sort();
    mods.dedup();
    if mods.len() > MAX_MODS {
        return Err(AppError::InvalidName("preset".into()));
    }
    match index(list, &name) {
        Some(i) => {
            list[i].mods = mods;
            list[i].name = name;
        }
        None => {
            if list.len() >= MAX_PRESETS {
                return Err(AppError::InvalidName("too many presets".into()));
            }
            list.push(Preset { name, mods });
        }
    }
    Ok(())
}

pub fn rename(list: &mut [Preset], old: &str, new: &str) -> Result<(), AppError> {
    let new = clean_name(new)?;
    let i = index(list, old).ok_or_else(|| AppError::NotFound(old.to_string()))?;
    if let Some(j) = index(list, &new) {
        if j != i {
            return Err(AppError::InvalidName(format!("{new} already exists")));
        }
    }
    list[i].name = new;
    Ok(())
}

pub fn delete(list: &mut Vec<Preset>, name: &str) -> Result<(), AppError> {
    let i = index(list, name).ok_or_else(|| AppError::NotFound(name.to_string()))?;
    list.remove(i);
    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Plan {
    pub enable: Vec<String>,
    pub disable: Vec<String>,
    pub missing: Vec<String>,
}

pub fn plan(preset: &Preset, records: &[ModRecord]) -> Plan {
    let mut p = Plan::default();
    for name in &preset.mods {
        match records.iter().find(|r| &r.name == name) {
            None => p.missing.push(name.clone()),
            Some(r) if !r.enabled => p.enable.push(name.clone()),
            Some(_) => {}
        }
    }
    for r in records {
        if r.enabled && !preset.mods.contains(&r.name) {
            p.disable.push(r.name.clone());
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(name: &str, enabled: bool) -> ModRecord {
        ModRecord { name: name.into(), enabled, ..ModRecord::default() }
    }

    #[test]
    fn presets_are_saved_overwritten_renamed_and_deleted_without_caring_about_case() {
        let mut list = Vec::new();
        upsert(&mut list, "  Story  run ", vec!["b".into(), "a".into(), "a".into()]).unwrap();
        assert_eq!(list, vec![Preset { name: "Story run".into(), mods: vec!["a".into(), "b".into()] }]);
        upsert(&mut list, "story RUN", vec!["c".into()]).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].name.as_str(), list[0].mods.clone()), ("story RUN", vec!["c".to_string()]));
        upsert(&mut list, "Other", vec![]).unwrap();
        assert!(rename(&mut list, "other", "STORY run").is_err());
        rename(&mut list, "other", "Co-op").unwrap();
        assert_eq!(list[1].name, "Co-op");
        assert!(rename(&mut list, "missing", "x").is_err());
        delete(&mut list, "co-op").unwrap();
        assert!(delete(&mut list, "co-op").is_err());
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn odd_names_and_too_many_presets_are_refused() {
        let mut list = Vec::new();
        for bad in ["", "   ", "a\u{0}b", &"x".repeat(61)] {
            assert!(upsert(&mut list, bad, vec![]).is_err(), "{bad:?}");
        }
        for i in 0..MAX_PRESETS {
            upsert(&mut list, &format!("p{i}"), vec![]).unwrap();
        }
        assert!(upsert(&mut list, "one more", vec![]).is_err());
        upsert(&mut list, "P3", vec!["x".into()]).unwrap();
        assert_eq!(list.len(), MAX_PRESETS);
    }

    #[test]
    fn presets_survive_a_restart_and_a_broken_file_reads_as_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load(tmp.path()).is_empty());
        let list = vec![Preset { name: "A".into(), mods: vec!["m".into()] }];
        save(tmp.path(), &list).unwrap();
        assert_eq!(load(tmp.path()), list);
        fs::write(file(tmp.path()), "{ nope").unwrap();
        assert!(load(tmp.path()).is_empty());
    }

    #[test]
    fn the_plan_turns_on_what_is_missing_from_the_game_and_off_what_is_not_wanted() {
        let records = vec![rec("a", true), rec("b", false), rec("c", true), rec("d", false)];
        let p = plan(&Preset { name: "x".into(), mods: vec!["a".into(), "b".into(), "gone".into()] }, &records);
        assert_eq!(p.enable, vec!["b".to_string()]);
        assert_eq!(p.disable, vec!["c".to_string()]);
        assert_eq!(p.missing, vec!["gone".to_string()]);
    }
}
