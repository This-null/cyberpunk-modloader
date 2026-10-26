use crate::error::AppError;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize)]
pub struct ReportInput {
    pub app_version: String,
    pub os: String,
    pub language: String,
    pub error: String,
    pub log_tail: Vec<String>,
    pub mods: Vec<String>,
    pub note: String,
}

fn replace_ci(text: &str, needle: &str, with: &str) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    let hay = text.to_ascii_lowercase();
    let n = needle.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(pos) = hay[i..].find(&n) {
        out.push_str(&text[i..i + pos]);
        out.push_str(with);
        i += pos + n.len();
    }
    out.push_str(&text[i..]);
    out
}

pub fn scrub_paths(text: &str, home: &str) -> String {
    let home = home.trim_end_matches(['\\', '/']);
    let escaped = home.replace('\\', "\\\\");
    let forward = home.replace('\\', "/");
    let step = replace_ci(text, &escaped, "%USERPROFILE%");
    let step = replace_ci(&step, home, "%USERPROFILE%");
    replace_ci(&step, &forward, "%USERPROFILE%")
}

pub fn build_report(input: &ReportInput, home: &str) -> String {
    let mut out = String::new();
    out.push_str("CP2077 Mod Launcher report\n\n");
    out.push_str(&format!("Version: {}\n", input.app_version));
    out.push_str(&format!("OS: {}\n", input.os));
    out.push_str(&format!("Language: {}\n\n", input.language));
    out.push_str("Error\n");
    out.push_str(&input.error);
    out.push_str("\n\n");
    if !input.note.trim().is_empty() {
        out.push_str("What the user was doing\n");
        out.push_str(input.note.trim());
        out.push_str("\n\n");
    }
    out.push_str("Mods\n");
    if input.mods.is_empty() {
        out.push_str("none enabled\n");
    }
    for m in &input.mods {
        out.push_str(&format!("- {m}\n"));
    }
    out.push_str("\nLog\n");
    for line in &input.log_tail {
        out.push_str(line);
        out.push('\n');
    }
    scrub_paths(&out, home)
}

fn timestamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        y,
        m,
        d,
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

pub fn save_report(text: &str, dir: &Path) -> Result<PathBuf, AppError> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("CP2077-ModLauncher-report-{}.txt", timestamp(secs)));
    fs::write(&path, text)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = "C:\\Users\\Ayse";

    fn input(error: &str) -> ReportInput {
        ReportInput {
            app_version: "0.1.0".into(),
            os: "Windows 11".into(),
            language: "tr".into(),
            error: error.into(),
            log_tail: vec!["line one".into()],
            mods: vec!["NightCoop 1.2".into()],
            note: String::new(),
        }
    }

    #[test]
    fn scrub_replaces_backslash_home() {
        assert_eq!(
            scrub_paths("open C:\\Users\\Ayse\\Desktop\\a.txt", HOME),
            "open %USERPROFILE%\\Desktop\\a.txt"
        );
    }

    #[test]
    fn scrub_replaces_forward_slash_home() {
        assert_eq!(
            scrub_paths("open C:/Users/Ayse/Desktop", HOME),
            "open %USERPROFILE%/Desktop"
        );
    }

    #[test]
    fn scrub_replaces_escaped_backslash_home() {
        assert_eq!(
            scrub_paths("Io(\"C:\\\\Users\\\\Ayse\\\\x\")", HOME),
            "Io(\"%USERPROFILE%\\\\x\")"
        );
    }

    #[test]
    fn scrub_is_case_insensitive() {
        assert_eq!(scrub_paths("c:\\users\\AYSE\\x", HOME), "%USERPROFILE%\\x");
    }

    #[test]
    fn scrub_handles_trailing_slash_in_home() {
        assert_eq!(scrub_paths("C:\\Users\\Ayse\\x", "C:\\Users\\Ayse\\"), "%USERPROFILE%\\x");
    }

    #[test]
    fn report_contains_version_error_and_mods() {
        let r = build_report(&input("boom"), HOME);
        assert!(r.contains("Version: 0.1.0"));
        assert!(r.contains("boom"));
        assert!(r.contains("- NightCoop 1.2"));
        assert!(r.contains("line one"));
    }

    #[test]
    fn report_never_contains_home() {
        let mut i = input("failed at C:\\Users\\Ayse\\AppData\\x");
        i.log_tail.push("read C:/Users/Ayse/y".into());
        i.note = "my folder is C:\\Users\\Ayse\\Games".into();
        let r = build_report(&i, HOME);
        assert!(!r.to_ascii_lowercase().contains("ayse"));
    }

    #[test]
    fn report_includes_note_only_when_present() {
        assert!(!build_report(&input("e"), HOME).contains("What the user was doing"));
        let mut i = input("e");
        i.note = "clicked enable".into();
        assert!(build_report(&i, HOME).contains("clicked enable"));
    }

    #[test]
    fn timestamp_formats_utc() {
        assert_eq!(timestamp(0), "19700101-000000");
        assert_eq!(timestamp(1_700_000_000), "20231114-221320");
    }

    #[test]
    fn save_writes_file_in_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let p = save_report("hello", tmp.path()).unwrap();
        assert!(p
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("CP2077-ModLauncher-report-"));
        assert_eq!(fs::read_to_string(p).unwrap(), "hello");
    }
}
