use crate::error::AppError;
use serde::Serialize;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const DEFAULT_CHUNK: u64 = 512 * 1024;
pub const MAX_CHUNK: u64 = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 4;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LogFile {
    pub id: String,
    pub source: String,
    pub group: String,
    pub name: String,
    pub size: u64,
    pub modified: u64,
    pub crash: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Chunk {
    pub text: String,
    pub start: u64,
    pub end: u64,
    pub size: u64,
}

fn modified_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn is_log(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("log"))
}

fn is_crash(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("crash") || lower.contains("hang")
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < MAX_DEPTH {
                walk(&path, depth + 1, out);
            }
        } else if is_log(&path) {
            out.push(path);
        }
    }
}

fn rel_id(prefix: &str, root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    Some(format!("{prefix}:{}", rel.to_string_lossy().replace('\\', "/")))
}

fn child_dirs(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn push(out: &mut Vec<LogFile>, prefix: &str, root: &Path, path: &Path, source: &str, group: &str) {
    let (Some(id), Ok(meta)) = (rel_id(prefix, root, path), fs::metadata(path)) else {
        return;
    };
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    out.push(LogFile {
        id,
        source: source.to_string(),
        group: group.to_string(),
        crash: is_crash(&name),
        name,
        size: meta.len(),
        modified: modified_secs(&meta),
    });
}

pub fn scan(game: Option<&Path>, data: &Path) -> Vec<LogFile> {
    let mut out = Vec::new();

    if let Some(game) = game {
        let mut files = Vec::new();
        walk(&game.join("red4ext").join("logs"), MAX_DEPTH, &mut files);
        let plugins = child_dirs(&game.join("red4ext").join("plugins"));
        for path in files {
            let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            let prefix = name.split(['-', '.', '_']).next().unwrap_or("").to_string();
            if prefix == "red4ext" {
                push(&mut out, "g", game, &path, "RED4ext", "loader");
            } else {
                let source = plugins
                    .iter()
                    .find(|p| p.eq_ignore_ascii_case(&prefix))
                    .cloned()
                    .unwrap_or(prefix);
                push(&mut out, "g", game, &path, &source, "mod");
            }
        }

        for plugin in &plugins {
            let mut files = Vec::new();
            walk(&game.join("red4ext").join("plugins").join(plugin), 1, &mut files);
            for path in files {
                push(&mut out, "g", game, &path, plugin, "mod");
            }
        }

        let mut files = Vec::new();
        walk(&game.join("r6").join("logs"), MAX_DEPTH, &mut files);
        for path in files {
            push(&mut out, "g", game, &path, "Redscript", "loader");
        }

        let cet = game.join("bin").join("x64").join("plugins").join("cyber_engine_tweaks");
        if let Ok(entries) = fs::read_dir(&cet) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_log(&path) {
                    push(&mut out, "g", game, &path, "Cyber Engine Tweaks", "loader");
                }
            }
        }
        for module in child_dirs(&cet.join("mods")) {
            let mut files = Vec::new();
            walk(&cet.join("mods").join(&module), 1, &mut files);
            for path in files {
                push(&mut out, "g", game, &path, &module, "mod");
            }
        }
    }

    let mut files = Vec::new();
    walk(&data.join("logs"), MAX_DEPTH, &mut files);
    for path in files {
        push(&mut out, "a", data, &path, "Launcher", "launcher");
    }

    out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.id.cmp(&b.id)));
    out.dedup_by(|a, b| a.id == b.id);
    out
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Cleared {
    pub removed: u32,
    pub freed: u64,
}

pub fn clear(game: Option<&Path>, data: &Path) -> Cleared {
    let mut done = Cleared { removed: 0, freed: 0 };
    for f in scan(game, data) {
        let Ok(path) = resolve(game, data, &f.id) else {
            continue;
        };
        if fs::remove_file(&path).is_ok() {
            done.removed += 1;
            done.freed += f.size;
        }
    }
    done
}

pub fn resolve(game: Option<&Path>, data: &Path, id: &str) -> Result<PathBuf, AppError> {
    let (prefix, rel) = id
        .split_once(':')
        .ok_or_else(|| AppError::UnsafePath(id.to_string()))?;
    let root = match prefix {
        "g" => game.ok_or(AppError::GameNotFound)?,
        "a" => data,
        _ => return Err(AppError::UnsafePath(id.to_string())),
    };
    let bad = rel.is_empty()
        || rel.starts_with('/')
        || rel.contains('\\')
        || rel.contains(':')
        || rel.split('/').any(|s| s.is_empty() || s == "." || s == "..")
        || !is_log(Path::new(rel));
    if bad {
        return Err(AppError::UnsafePath(id.to_string()));
    }
    let path = root.join(rel);
    let canonical_root = fs::canonicalize(root)?;
    let canonical = fs::canonicalize(&path).map_err(|_| AppError::NotFound(id.to_string()))?;
    if !canonical.starts_with(&canonical_root) || !canonical.is_file() {
        return Err(AppError::UnsafePath(id.to_string()));
    }
    Ok(path)
}

fn read_range(path: &Path, start: u64, end: u64) -> Result<Vec<u8>, AppError> {
    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    file.take(end.saturating_sub(start)).read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn read_chunk(path: &Path, end: Option<u64>, max: u64) -> Result<Chunk, AppError> {
    let size = fs::metadata(path)?.len();
    let end = end.unwrap_or(size).min(size);
    let max = max.clamp(1, MAX_CHUNK);
    let mut start = end.saturating_sub(max);
    let mut bytes = read_range(path, start, end)?;
    if start > 0 {
        match bytes.iter().position(|&b| b == b'\n') {
            Some(i) => {
                bytes.drain(..=i);
                start += i as u64 + 1;
            }
            None => {
                start = end;
                bytes.clear();
            }
        }
    }
    Ok(Chunk {
        text: String::from_utf8_lossy(&bytes).into_owned(),
        start,
        end,
        size,
    })
}

pub fn read_head(path: &Path, max: u64) -> Result<Chunk, AppError> {
    let size = fs::metadata(path)?.len();
    let end = size.min(max.clamp(1, MAX_CHUNK));
    let mut bytes = read_range(path, 0, end)?;
    let mut real_end = end;
    if end < size {
        if let Some(i) = bytes.iter().rposition(|&b| b == b'\n') {
            bytes.truncate(i + 1);
            real_end = i as u64 + 1;
        }
    }
    Ok(Chunk {
        text: String::from_utf8_lossy(&bytes).into_owned(),
        start: 0,
        end: real_end,
        size,
    })
}

fn utf8_prefix_len(bytes: &[u8]) -> usize {
    match std::str::from_utf8(bytes) {
        Ok(_) => bytes.len(),
        Err(e) if e.error_len().is_none() => e.valid_up_to(),
        Err(_) => bytes.len(),
    }
}

pub fn read_since(path: &Path, from: u64, max: u64) -> Result<Chunk, AppError> {
    let size = fs::metadata(path)?.len();
    if size < from {
        return read_chunk(path, None, max);
    }
    let end = size.min(from + max.clamp(1, MAX_CHUNK));
    let mut bytes = read_range(path, from, end)?;
    let keep = utf8_prefix_len(&bytes);
    bytes.truncate(keep);
    let end = from + keep as u64;
    Ok(Chunk {
        text: String::from_utf8_lossy(&bytes).into_owned(),
        start: from,
        end,
        size,
    })
}

pub fn excerpt(path: &Path, lines: usize) -> Result<String, AppError> {
    let bytes = read_range(path, 0, 64 * 1024)?;
    let text = String::from_utf8_lossy(&bytes);
    let picked: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).take(lines).collect();
    Ok(picked.join("\n").chars().take(4000).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    fn put(root: &Path, rel: &str, body: &str) -> PathBuf {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, body).unwrap();
        p
    }

    fn age(path: &Path, secs_ago: u64) {
        let t = SystemTime::now() - Duration::from_secs(secs_ago);
        File::options().write(true).open(path).unwrap().set_modified(t).unwrap();
    }

    fn find<'a>(files: &'a [LogFile], name: &str) -> &'a LogFile {
        files.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("missing {name}"))
    }

    #[test]
    fn finds_logs_with_their_source_and_group() {
        let game = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        put(game.path(), "red4ext/plugins/NightCoop/x.dll", "");
        put(game.path(), "red4ext/logs/red4ext-1.log", "a");
        put(game.path(), "red4ext/logs/nightcoop-1.log", "b");
        put(game.path(), "red4ext/plugins/NightCoop/logs/nightcoop-session-1.log", "c");
        put(game.path(), "red4ext/plugins/NightCoop/nightcoop.log", "d");
        put(game.path(), "r6/logs/redscript_rCURRENT.log", "e");
        put(game.path(), "bin/x64/plugins/cyber_engine_tweaks/cyber_engine_tweaks.log", "f");
        put(game.path(), "bin/x64/plugins/cyber_engine_tweaks/mods/Cool/cool.log", "g");
        put(game.path(), "red4ext/plugins/NightCoop/readme.txt", "ignored");
        put(data.path(), "logs/launcher.2026-10-26.log", "h");

        let files = scan(Some(game.path()), data.path());
        assert_eq!(files.len(), 8);
        assert_eq!(find(&files, "red4ext-1.log").source, "RED4ext");
        assert_eq!(find(&files, "red4ext-1.log").group, "loader");
        assert_eq!(find(&files, "nightcoop-1.log").source, "NightCoop");
        assert_eq!(find(&files, "nightcoop-1.log").group, "mod");
        assert_eq!(find(&files, "nightcoop-session-1.log").source, "NightCoop");
        assert_eq!(find(&files, "redscript_rCURRENT.log").source, "Redscript");
        assert_eq!(find(&files, "cyber_engine_tweaks.log").source, "Cyber Engine Tweaks");
        assert_eq!(find(&files, "cool.log").source, "Cool");
        assert_eq!(find(&files, "cool.log").group, "mod");
        assert_eq!(find(&files, "launcher.2026-10-26.log").group, "launcher");
        assert!(files[0].id.contains(':'));
    }

    #[test]
    fn flags_crash_and_hang_logs_and_sorts_newest_first() {
        let game = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        let old = put(game.path(), "red4ext/plugins/NightCoop/logs/a-exit.log", "x");
        let crash = put(game.path(), "red4ext/plugins/NightCoop/logs/nightcoop-last-1-crash.log", "x");
        let hang = put(game.path(), "red4ext/plugins/NightCoop/logs/nightcoop-last-2-hang.log", "x");
        age(&old, 500);
        age(&crash, 100);
        age(&hang, 10);
        let files = scan(Some(game.path()), data.path());
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["nightcoop-last-2-hang.log", "nightcoop-last-1-crash.log", "a-exit.log"]);
        assert!(files[0].crash && files[1].crash && !files[2].crash);
    }

    #[test]
    fn works_without_a_game_folder() {
        let data = tempfile::tempdir().unwrap();
        put(data.path(), "logs/launcher.log", "x");
        let files = scan(None, data.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].group, "launcher");
        assert!(scan(None, &data.path().join("nothing")).is_empty());
    }

    #[test]
    fn clearing_removes_listed_logs_and_nothing_else() {
        let game = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        put(game.path(), "red4ext/logs/red4ext-1.log", "hello");
        put(game.path(), "red4ext/logs/keep.txt", "notes");
        put(data.path(), "logs/l.log", "abc");
        let done = clear(Some(game.path()), data.path());
        assert_eq!((done.removed, done.freed), (2, 8));
        assert!(!game.path().join("red4ext/logs/red4ext-1.log").exists());
        assert!(game.path().join("red4ext/logs/keep.txt").exists());
        assert!(scan(Some(game.path()), data.path()).is_empty());
    }

    #[test]
    fn resolve_accepts_listed_files_only() {
        let game = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        put(game.path(), "red4ext/logs/red4ext-1.log", "x");
        put(data.path(), "logs/l.log", "x");
        let id = scan(Some(game.path()), data.path()).into_iter().find(|f| f.name == "red4ext-1.log").unwrap().id;
        assert!(resolve(Some(game.path()), data.path(), &id).is_ok());
        assert!(resolve(Some(game.path()), data.path(), "a:logs/l.log").is_ok());
        for bad in [
            "g:../secret.log",
            "g:red4ext/../../x.log",
            "g:/abs.log",
            "g:C:/x.log",
            "g:red4ext\\logs\\x.log",
            "x:red4ext/logs/red4ext-1.log",
            "nocolon",
            "g:red4ext/logs/red4ext-1.txt",
            "g:",
            "g:red4ext//logs/red4ext-1.log",
        ] {
            assert!(
                matches!(resolve(Some(game.path()), data.path(), bad), Err(AppError::UnsafePath(_))),
                "{bad}"
            );
        }
        assert!(matches!(
            resolve(Some(game.path()), data.path(), "g:red4ext/logs/missing.log"),
            Err(AppError::NotFound(_))
        ));
        assert!(matches!(resolve(None, data.path(), "g:red4ext/logs/red4ext-1.log"), Err(AppError::GameNotFound)));
    }

    #[test]
    fn small_files_are_returned_whole() {
        let tmp = tempfile::tempdir().unwrap();
        let p = put(tmp.path(), "a.log", "one\ntwo\nthree\n");
        let c = read_chunk(&p, None, DEFAULT_CHUNK).unwrap();
        assert_eq!(c.text, "one\ntwo\nthree\n");
        assert_eq!((c.start, c.end, c.size), (0, 14, 14));
    }

    #[test]
    fn big_files_are_read_from_the_end_on_a_line_boundary() {
        let tmp = tempfile::tempdir().unwrap();
        let body: String = (0..1000).map(|i| format!("line {i:04}\n")).collect();
        let p = put(tmp.path(), "big.log", &body);
        let c = read_chunk(&p, None, 100).unwrap();
        assert!(c.start > 0);
        assert_eq!(c.end, c.size);
        assert!(c.text.starts_with("line "));
        assert!(c.text.ends_with("line 0999\n"));
        assert!(c.text.len() <= 100);
        let earlier = read_chunk(&p, Some(c.start), 100).unwrap();
        assert_eq!(earlier.end, c.start);
        assert_eq!(format!("{}{}", earlier.text, c.text), body[earlier.start as usize..]);
    }

    #[test]
    fn a_line_longer_than_the_chunk_yields_nothing_instead_of_garbage() {
        let tmp = tempfile::tempdir().unwrap();
        let p = put(tmp.path(), "long.log", &"x".repeat(500));
        let c = read_chunk(&p, None, 100).unwrap();
        assert_eq!(c.text, "");
    }

    #[test]
    fn invalid_utf8_does_not_fail() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("bin.log");
        fs::write(&p, [b'a', b'\n', 0xff, 0xfe, b'\n']).unwrap();
        let c = read_chunk(&p, None, DEFAULT_CHUNK).unwrap();
        assert!(c.text.starts_with("a\n"));
    }

    #[test]
    fn polling_returns_only_new_text_and_survives_truncation() {
        let tmp = tempfile::tempdir().unwrap();
        let p = put(tmp.path(), "a.log", "one\n");
        let first = read_chunk(&p, None, DEFAULT_CHUNK).unwrap();
        let mut f = fs::OpenOptions::new().append(true).open(&p).unwrap();
        std::io::Write::write_all(&mut f, b"two\n").unwrap();
        let more = read_since(&p, first.end, DEFAULT_CHUNK).unwrap();
        assert_eq!(more.text, "two\n");
        assert_eq!(more.end, 8);
        let none = read_since(&p, more.end, DEFAULT_CHUNK).unwrap();
        assert_eq!(none.text, "");
        fs::write(&p, "new\n").unwrap();
        let restarted = read_since(&p, 8, DEFAULT_CHUNK).unwrap();
        assert_eq!(restarted.text, "new\n");
        assert_eq!(restarted.start, 0);
    }

    #[test]
    fn the_head_stops_on_a_line_boundary_and_can_be_continued() {
        let tmp = tempfile::tempdir().unwrap();
        let body: String = (0..1000).map(|i| format!("line {i:04}\n")).collect();
        let p = put(tmp.path(), "big.log", &body);
        let head = read_head(&p, 100).unwrap();
        assert_eq!((head.start, head.size), (0, body.len() as u64));
        assert!(head.text.starts_with("line 0000\n"));
        assert!(head.text.ends_with('\n'));
        assert!(head.end <= 100);
        let next = read_since(&p, head.end, 100).unwrap();
        assert_eq!(format!("{}{}", head.text, next.text), body[..next.end as usize]);
    }

    #[test]
    fn a_small_file_is_one_head_chunk() {
        let tmp = tempfile::tempdir().unwrap();
        let p = put(tmp.path(), "a.log", "one\ntwo");
        let head = read_head(&p, DEFAULT_CHUNK).unwrap();
        assert_eq!(head.text, "one\ntwo");
        assert_eq!(head.end, head.size);
    }

    #[test]
    fn a_character_split_between_two_polls_arrives_whole() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("u.log");
        fs::write(&p, "aé\n".as_bytes()).unwrap();
        let first = read_since(&p, 0, 2).unwrap();
        assert_eq!((first.text.as_str(), first.end), ("a", 1));
        let second = read_since(&p, first.end, 100).unwrap();
        assert_eq!(second.text, "é\n");
        assert_eq!(format!("{}{}", first.text, second.text), "aé\n");
    }

    #[test]
    fn excerpt_takes_the_first_non_empty_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let p = put(tmp.path(), "c.log", "\nerror one\n\nerror two\nthree\n");
        assert_eq!(excerpt(&p, 2).unwrap(), "error one\nerror two");
    }
}
