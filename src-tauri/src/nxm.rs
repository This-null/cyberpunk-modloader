use crate::archive;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NxmLink {
    pub mod_id: u64,
    pub file_id: u64,
    pub key: String,
    pub expires: u64,
}

fn number(s: &str) -> Option<u64> {
    (!s.is_empty() && s.len() <= 12 && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

pub fn parse(raw: &str) -> Result<NxmLink, AppError> {
    let bad = || AppError::BadLink("nxm".into());
    if raw.len() > 600 || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(bad());
    }
    let rest = raw
        .get(..6)
        .filter(|p| p.eq_ignore_ascii_case("nxm://"))
        .map(|_| &raw[6..])
        .ok_or_else(bad)?;
    let (path, query) = rest.split_once('?').ok_or_else(bad)?;
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    let [game, "mods", mod_id, "files", file_id] = parts[..] else {
        return Err(bad());
    };
    if !game.eq_ignore_ascii_case(crate::nexus::GAME) {
        return Err(bad());
    }
    let (mut key, mut expires) = (None, None);
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("key", v)) => key = Some(v),
            Some(("expires", v)) => expires = Some(v),
            _ => {}
        }
    }
    let key = key
        .filter(|k| (1..=200).contains(&k.len()) && k.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)))
        .ok_or_else(bad)?;
    Ok(NxmLink {
        mod_id: number(mod_id).ok_or_else(bad)?,
        file_id: number(file_id).ok_or_else(bad)?,
        key: key.to_string(),
        expires: expires.and_then(number).ok_or_else(bad)?,
    })
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Handler {
    Ours,
    Other,
    None,
}

fn command_for(exe: &Path) -> String {
    format!("\"{}\" \"%1\"", exe.display())
}

pub fn classify(current: Option<&str>, exe: &Path) -> Handler {
    match current {
        None => Handler::None,
        Some(c) if c.to_ascii_lowercase().contains(&exe.display().to_string().to_ascii_lowercase()) => Handler::Ours,
        Some(_) => Handler::Other,
    }
}

#[cfg(windows)]
mod registry {
    use super::*;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    const ROOT: &str = r"Software\Classes\nxm";

    pub fn read() -> Option<String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(format!(r"{ROOT}\shell\open\command"), KEY_READ)
            .ok()?
            .get_value::<String, _>("")
            .ok()
    }

    pub fn write(exe: &Path) -> Result<(), AppError> {
        let io = |e: std::io::Error| AppError::Io(e.to_string());
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (root, _) = hkcu.create_subkey(ROOT).map_err(io)?;
        root.set_value("", &"URL:NXM Protocol").map_err(io)?;
        root.set_value("URL Protocol", &"").map_err(io)?;
        let (cmd, _) = hkcu.create_subkey(format!(r"{ROOT}\shell\open\command")).map_err(io)?;
        cmd.set_value("", &command_for(exe)).map_err(io)
    }

    pub fn remove() -> Result<(), AppError> {
        RegKey::predef(HKEY_CURRENT_USER)
            .delete_subkey_all(ROOT)
            .map_err(|e| AppError::Io(e.to_string()))
    }
}

#[cfg(not(windows))]
mod registry {
    use super::*;

    pub fn read() -> Option<String> {
        None
    }

    pub fn write(_: &Path) -> Result<(), AppError> {
        Err(AppError::Io("nxm links are only supported on Windows".into()))
    }

    pub fn remove() -> Result<(), AppError> {
        Ok(())
    }
}

pub fn status(exe: &Path) -> Handler {
    classify(registry::read().as_deref(), exe)
}

pub fn register(exe: &Path) -> Result<(), AppError> {
    registry::write(exe)
}

pub fn unregister(exe: &Path) -> Result<(), AppError> {
    if status(exe) == Handler::Ours {
        registry::remove()?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Found {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub modified: u64,
}

const SETTLE_SECS: u64 = 3;

pub fn recent_archives(dir: &Path, since: u64, now: u64) -> Vec<Found> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<Found> = read
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let meta = e.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            let fine = meta.is_file()
                && meta.len() > 0
                && archive::kind(&path).is_some()
                && modified >= since
                && now >= modified + SETTLE_SECS;
            let name = path.file_name()?.to_string_lossy().into_owned();
            fine.then(|| Found {
                name,
                path: path.to_string_lossy().into_owned(),
                size: meta.len(),
                modified,
            })
        })
        .collect();
    out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));
    out.truncate(10);
    out
}

pub fn is_inside(dir: &Path, file: &Path) -> bool {
    match (dir.canonicalize(), file.canonicalize()) {
        (Ok(d), Ok(f)) => f.starts_with(d) && f.is_file(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const OK: &str = "nxm://cyberpunk2077/mods/107/files/4521?key=AbC-123_x&expires=1790000000&user_id=9";

    #[test]
    fn a_normal_link_is_understood() {
        let l = parse(OK).unwrap();
        assert_eq!((l.mod_id, l.file_id, l.expires), (107, 4521, 1_790_000_000));
        assert_eq!(l.key, "AbC-123_x");
        assert!(parse("NXM://Cyberpunk2077/mods/1/files/2/?expires=5&key=k").is_ok());
    }

    #[test]
    fn odd_links_are_refused() {
        for bad in [
            "",
            "https://cyberpunk2077/mods/1/files/2?key=k&expires=5",
            "nxm://skyrim/mods/1/files/2?key=k&expires=5",
            "nxm://cyberpunk2077/mods/1/files?key=k&expires=5",
            "nxm://cyberpunk2077/mods/x/files/2?key=k&expires=5",
            "nxm://cyberpunk2077/mods/1/files/2",
            "nxm://cyberpunk2077/mods/1/files/2?expires=5",
            "nxm://cyberpunk2077/mods/1/files/2?key=k",
            "nxm://cyberpunk2077/mods/1/files/2?key=a b&expires=5",
            "nxm://cyberpunk2077/mods/1/files/2?key=a%26b&expires=5",
            "nxm://cyberpunk2077/mods/1/files/2?key=k&expires=-5",
            "nxm://cyberpunk2077/mods/1/files/2?key=k&expires=5\n",
            "nxm://cyberpunk2077/mods/../files/2?key=k&expires=5",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        let long = format!("nxm://cyberpunk2077/mods/1/files/2?key={}&expires=5", "a".repeat(700));
        assert!(parse(&long).is_err());
    }

    #[test]
    fn the_handler_is_ours_only_when_it_points_at_this_exe() {
        let exe = Path::new(r"D:\Apps\CP2077 Mod Launcher\launcher.exe");
        assert_eq!(classify(None, exe), Handler::None);
        assert_eq!(classify(Some(&command_for(exe)), exe), Handler::Ours);
        assert_eq!(classify(Some("\"c:\\vortex\\Vortex.exe\" -d \"%1\""), exe), Handler::Other);
        assert_eq!(command_for(exe), "\"D:\\Apps\\CP2077 Mod Launcher\\launcher.exe\" \"%1\"");
    }

    fn put(dir: &Path, name: &str, body: &str, age_secs: u64) {
        let p = dir.join(name);
        fs::write(&p, body).unwrap();
        let t = SystemTime::now() - Duration::from_secs(age_secs);
        fs::File::options().write(true).open(&p).unwrap().set_modified(t).unwrap();
    }

    #[test]
    fn only_settled_recent_archives_are_listed_newest_first() {
        let tmp = tempfile::tempdir().unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        put(tmp.path(), "old.zip", "x", 3600);
        put(tmp.path(), "new.rar", "x", 20);
        put(tmp.path(), "newer.7z", "x", 10);
        put(tmp.path(), "busy.zip", "x", 0);
        put(tmp.path(), "notes.txt", "x", 10);
        put(tmp.path(), "empty.zip", "", 10);
        put(tmp.path(), "part.zip.crdownload", "x", 10);
        let found = recent_archives(tmp.path(), now - 600, now);
        let names: Vec<_> = found.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["newer.7z", "new.rar"]);
        assert!(recent_archives(&tmp.path().join("missing"), 0, now).is_empty());
    }

    #[test]
    fn a_file_must_really_be_inside_the_downloads_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dl = tmp.path().join("dl");
        fs::create_dir_all(&dl).unwrap();
        fs::write(dl.join("a.zip"), "x").unwrap();
        fs::write(tmp.path().join("b.zip"), "x").unwrap();
        assert!(is_inside(&dl, &dl.join("a.zip")));
        assert!(!is_inside(&dl, &tmp.path().join("b.zip")));
        assert!(!is_inside(&dl, &dl.join("..").join("b.zip")));
        assert!(!is_inside(&dl, &dl.join("nope.zip")));
        assert!(!is_inside(&dl, &dl));
    }
}
