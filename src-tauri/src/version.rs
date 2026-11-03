use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_DLL: u64 = 256 * 1024 * 1024;
const LOG_HEAD: u64 = 64 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct BuildInfo {
    pub version: String,
    pub build: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Detected {
    pub version: Option<String>,
    pub build: Option<String>,
    pub source: String,
}

fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_file() && p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name)))
}

fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

pub fn contains_token(data: &[u8], token: &[u8]) -> bool {
    if token.is_empty() || data.len() < token.len() {
        return false;
    }
    let mut from = 0;
    while let Some(pos) = data[from..].windows(token.len()).position(|w| w == token) {
        let at = from + pos;
        let before_ok = at == 0 || !is_token_byte(data[at - 1]);
        let after = at + token.len();
        let after_ok = after >= data.len() || !is_token_byte(data[after]);
        if before_ok && after_ok {
            return true;
        }
        from = at + 1;
    }
    false
}

fn from_dll(dll: &Path, builds: &[BuildInfo]) -> Option<BuildInfo> {
    if fs::metadata(dll).ok()?.len() > MAX_DLL {
        return None;
    }
    let data = fs::read(dll).ok()?;
    let mut hits = builds
        .iter()
        .filter(|b| b.build.len() >= 6 && contains_token(&data, b.build.as_bytes()));
    let first = hits.next()?;
    if hits.next().is_some() {
        return None;
    }
    Some(first.clone())
}

fn from_log(log: &Path) -> Option<String> {
    let mut buf = Vec::new();
    fs::File::open(log).ok()?.take(LOG_HEAD).read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    let re = Regex::new(r"(?i)\bbuild ([0-9a-f]{6,40})\b").ok()?;
    re.captures(&text).map(|c| c[1].to_ascii_lowercase())
}

pub fn detect_nightcoop(game: &Path, builds: &[BuildInfo]) -> Option<Detected> {
    let dir = game.join("red4ext").join("plugins").join("NightCoop");
    let dll = find_ci(&dir, "NightCoop.dll")?;
    if let Some(hit) = from_dll(&dll, builds) {
        return Some(Detected {
            version: Some(hit.version),
            build: Some(hit.build),
            source: "dll".to_string(),
        });
    }
    if let Some(log) = find_ci(&dir, "nightcoop.log") {
        if let Some(hash) = from_log(&log) {
            let known = builds.iter().find(|b| b.build.eq_ignore_ascii_case(&hash));
            return Some(Detected {
                version: known.map(|b| b.version.clone()),
                build: Some(hash),
                source: "log".to_string(),
            });
        }
    }
    Some(Detected {
        version: None,
        build: None,
        source: "none".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builds() -> Vec<BuildInfo> {
        vec![
            BuildInfo { version: "0.1.1-beta.2".into(), build: "8b2fe3e".into() },
            BuildInfo { version: "0.1.0-beta.1".into(), build: "a7c537d".into() },
        ]
    }

    fn install(game: &Path, dll: &[u8], log: Option<&str>) {
        let dir = game.join("red4ext").join("plugins").join("NightCoop");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("NightCoop.dll"), dll).unwrap();
        if let Some(log) = log {
            fs::write(dir.join("nightcoop.log"), log).unwrap();
        }
    }

    #[test]
    fn token_matching_respects_word_edges() {
        assert!(contains_token(b"xx\x008b2fe3e\x00Oct", b"8b2fe3e"));
        assert!(contains_token(b"8b2fe3e", b"8b2fe3e"));
        assert!(!contains_token(b"aa8b2fe3eb", b"8b2fe3e"));
        assert!(!contains_token(b"8b2fe3ef", b"8b2fe3e"));
        assert!(contains_token(b"zz8b2fe3eb 8b2fe3e.", b"8b2fe3e"));
        assert!(!contains_token(b"", b"8b2fe3e"));
        assert!(!contains_token(b"abc", b""));
    }

    #[test]
    fn reads_the_version_from_the_dll() {
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path(), b"MZ....\x008b2fe3e\x00Oct 24 2026 17:15:29\x00....", None);
        let d = detect_nightcoop(tmp.path(), &builds()).unwrap();
        assert_eq!(d.version.as_deref(), Some("0.1.1-beta.2"));
        assert_eq!(d.build.as_deref(), Some("8b2fe3e"));
        assert_eq!(d.source, "dll");
    }

    #[test]
    fn picks_the_older_build_when_only_it_is_present() {
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path(), b"\x00a7c537d\x00", None);
        let d = detect_nightcoop(tmp.path(), &builds()).unwrap();
        assert_eq!(d.version.as_deref(), Some("0.1.0-beta.1"));
    }

    #[test]
    fn falls_back_to_the_log_when_the_dll_has_no_known_build() {
        let tmp = tempfile::tempdir().unwrap();
        install(
            tmp.path(),
            b"MZ nothing here",
            Some("[22:27:04.281] [info ] NightCoop loading, protocol version 128, build a7c537d (Oct 23)\n"),
        );
        let d = detect_nightcoop(tmp.path(), &builds()).unwrap();
        assert_eq!(d.version.as_deref(), Some("0.1.0-beta.1"));
        assert_eq!(d.source, "log");
    }

    #[test]
    fn an_unknown_build_keeps_its_hash_but_has_no_version() {
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path(), b"MZ", Some("NightCoop loading, protocol version 130, build deadbeef (x)\n"));
        let d = detect_nightcoop(tmp.path(), &builds()).unwrap();
        assert_eq!(d.version, None);
        assert_eq!(d.build.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn two_builds_in_one_dll_are_not_guessed() {
        let tmp = tempfile::tempdir().unwrap();
        install(tmp.path(), b"\x008b2fe3e\x00 \x00a7c537d\x00", None);
        let d = detect_nightcoop(tmp.path(), &builds()).unwrap();
        assert_eq!(d.version, None);
        assert_eq!(d.source, "none");
    }

    #[test]
    fn nothing_installed_means_nothing_detected() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(detect_nightcoop(tmp.path(), &builds()), None);
    }

    #[test]
    fn the_plugin_folder_and_file_names_are_matched_without_case() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("red4ext").join("plugins").join("NightCoop");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("nightcoop.DLL"), b"\x008b2fe3e\x00").unwrap();
        assert_eq!(detect_nightcoop(tmp.path(), &builds()).unwrap().version.as_deref(), Some("0.1.1-beta.2"));
    }
}
