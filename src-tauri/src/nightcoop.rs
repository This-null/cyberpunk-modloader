use crate::error::AppError;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const OFFICIAL_BASE: &str = "https://nightcoop.net/";
pub const RELEASES_PATH: &str = "releases.json";
const MAX_MANIFEST: u64 = 2 * 1024 * 1024;
const SLACK: u64 = 1024 * 1024;

#[derive(Clone)]
pub struct Endpoint {
    pub base: String,
    pub host: String,
    pub allow_http: bool,
}

impl Endpoint {
    pub fn official() -> Endpoint {
        Endpoint {
            base: OFFICIAL_BASE.to_string(),
            host: "nightcoop.net".to_string(),
            allow_http: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseFile {
    #[serde(default)]
    pub kind: String,
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub game: Vec<String>,
    #[serde(default)]
    pub protocol: u32,
    #[serde(default)]
    pub build: String,
    #[serde(default)]
    pub title: BTreeMap<String, String>,
    #[serde(default)]
    pub notes: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub files: Vec<ReleaseFile>,
    #[serde(default)]
    pub yanked: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Releases {
    pub schema: u32,
    #[serde(default)]
    pub updated: String,
    #[serde(default)]
    pub latest: String,
    pub releases: Vec<Release>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReleaseView {
    pub version: String,
    pub channel: String,
    pub date: String,
    pub game: Vec<String>,
    pub protocol: u32,
    pub build: String,
    pub title: String,
    pub notes: Vec<String>,
    pub size: u64,
    pub sha256: String,
    pub file_name: String,
    pub yanked: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReleasesView {
    pub updated: String,
    pub latest: String,
    pub releases: Vec<ReleaseView>,
}

fn net(e: impl Display) -> AppError {
    AppError::Network(e.to_string())
}

pub fn parse_releases(text: &str) -> Result<Releases, AppError> {
    let parsed: Releases = serde_json::from_str(text).map_err(|_| net("the release list is not valid"))?;
    if parsed.schema != 1 {
        return Err(net("the release list uses an unsupported version"));
    }
    Ok(parsed)
}

fn pick<'a, T>(map: &'a BTreeMap<String, T>, lang: &str) -> Option<&'a T> {
    map.get(lang).or_else(|| map.get("en")).or_else(|| map.values().next())
}

fn yanked_reason(value: &Value, lang: &str) -> Option<String> {
    match value {
        Value::Null | Value::Bool(false) => None,
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Object(map) => match map.get("reason") {
            Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Some(Value::Object(by_lang)) => by_lang
                .get(lang)
                .or_else(|| by_lang.get("en"))
                .or_else(|| by_lang.values().next())
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string()),
            _ => Some("withdrawn".to_string()),
        },
        _ => Some("withdrawn".to_string()),
    }
}

pub fn mod_file(release: &Release) -> Option<&ReleaseFile> {
    release
        .files
        .iter()
        .find(|f| f.kind == "mod")
        .or_else(|| release.files.first())
}

pub fn to_view(releases: &Releases, lang: &str) -> ReleasesView {
    let views = releases
        .releases
        .iter()
        .filter_map(|r| {
            let file = mod_file(r)?;
            Some(ReleaseView {
                version: r.version.clone(),
                channel: r.channel.clone(),
                date: r.date.clone(),
                game: r.game.clone(),
                protocol: r.protocol,
                build: r.build.clone(),
                title: pick(&r.title, lang).cloned().unwrap_or_default(),
                notes: pick(&r.notes, lang).cloned().unwrap_or_default(),
                size: file.size,
                sha256: file.sha256.to_ascii_lowercase(),
                file_name: file.name.clone(),
                yanked: r.yanked.as_ref().and_then(|v| yanked_reason(v, lang)),
            })
        })
        .collect();
    ReleasesView {
        updated: releases.updated.clone(),
        latest: releases.latest.clone(),
        releases: views,
    }
}

pub fn resolve_url(endpoint: &Endpoint, rel: &str) -> Result<String, AppError> {
    let ok = !rel.is_empty()
        && !rel.starts_with('/')
        && !rel.contains("..")
        && !rel.contains("//")
        && !rel.contains(':')
        && rel
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/+".contains(c));
    if ok {
        Ok(format!("{}{}", endpoint.base, rel))
    } else {
        Err(AppError::UnsafePath(rel.to_string()))
    }
}

fn client(endpoint: &Endpoint) -> Result<Client, AppError> {
    let host = endpoint.host.clone();
    let allow_http = endpoint.allow_http;
    Client::builder()
        .user_agent(format!("CP2077-Mod-Launcher/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .https_only(!allow_http)
        .redirect(Policy::custom(move |attempt| {
            let same_host = attempt.url().host_str() == Some(host.as_str());
            let scheme_ok = attempt.url().scheme() == "https" || allow_http;
            if attempt.previous().len() >= 3 {
                attempt.error("too many redirects")
            } else if same_host && scheme_ok {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(net)
}

pub fn fetch_text(endpoint: &Endpoint, rel: &str) -> Result<String, AppError> {
    let url = resolve_url(endpoint, rel)?;
    let response = client(endpoint)?.get(url).send().map_err(net)?;
    if !response.status().is_success() {
        return Err(net(format!("the server answered {}", response.status())));
    }
    let mut buf = Vec::new();
    response.take(MAX_MANIFEST + 1).read_to_end(&mut buf).map_err(net)?;
    if buf.len() as u64 > MAX_MANIFEST {
        return Err(net("the release list is too large"));
    }
    String::from_utf8(buf).map_err(|_| net("the release list is not valid text"))
}

pub fn download(
    endpoint: &Endpoint,
    rel: &str,
    dest: &Path,
    expected_size: u64,
    expected_sha256: &str,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(), AppError> {
    let url = resolve_url(endpoint, rel)?;
    let mut response = client(endpoint)?.get(url).send().map_err(net)?;
    if !response.status().is_success() {
        return Err(net(format!("the server answered {}", response.status())));
    }
    if response.content_length().is_some_and(|len| len > expected_size + SLACK) {
        return Err(net("the download is larger than announced"));
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let part: PathBuf = dest.with_extension("part");
    let result = (|| -> Result<(), AppError> {
        let mut out = fs::File::create(&part)?;
        let mut hasher = Sha256::new();
        let mut done = 0u64;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = response.read(&mut buf).map_err(net)?;
            if n == 0 {
                break;
            }
            done += n as u64;
            if done > expected_size + SLACK {
                return Err(net("the download is larger than announced"));
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n])?;
            progress(done, expected_size);
        }
        out.flush()?;
        let digest = hasher.finalize();
        let hex: String = digest.as_slice().iter().map(|b| format!("{b:02x}")).collect();
        if done != expected_size || !hex.eq_ignore_ascii_case(expected_sha256.trim()) {
            return Err(AppError::Checksum);
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            fs::rename(&part, dest)?;
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&part);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::thread;

    type Reply = (u16, Vec<(String, String)>, Vec<u8>);

    fn serve(handler: impl Fn(&str) -> Reply + Send + 'static) -> Endpoint {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }
                let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, headers, body) = handler(&path);
                let mut head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n",
                    body.len()
                );
                for (k, v) in headers {
                    head.push_str(&format!("{k}: {v}\r\n"));
                }
                head.push_str("\r\n");
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        Endpoint {
            base: format!("http://127.0.0.1:{port}/"),
            host: "127.0.0.1".to_string(),
            allow_http: true,
        }
    }

    fn sha(bytes: &[u8]) -> String {
        Sha256::digest(bytes).as_slice().iter().map(|b| format!("{b:02x}")).collect()
    }

    const SAMPLE: &str = r#"{
      "schema": 1, "updated": "2026-10-24T15:19:21Z", "latest": "0.1.1-beta.2",
      "releases": [
        {"version":"0.1.1-beta.2","channel":"beta","date":"2026-10-24","game":["2.5","2.31"],"protocol":128,"build":"8b2fe3e",
         "title":{"ru":"Вторая бета","en":"The second beta"},
         "notes":{"ru":["заметка"],"en":["a note"]},
         "files":[{"kind":"mod","name":"NightCoop-0.1.1-beta.2.zip","url":"dl/0.1.1-beta.2/NightCoop-0.1.1-beta.2.zip","size":10,"sha256":"ABCD"}],
         "yanked":null},
        {"version":"0.1.0-beta.1","channel":"beta","date":"2026-10-23","protocol":128,
         "files":[{"kind":"mod","name":"a.zip","url":"dl/a.zip","size":1,"sha256":"ff"}],
         "yanked":"broke saves"},
        {"version":"0.0.9","files":[]}
      ]}"#;

    #[test]
    fn parses_the_real_manifest_shape() {
        let r = parse_releases(SAMPLE).unwrap();
        assert_eq!(r.latest, "0.1.1-beta.2");
        assert_eq!(r.releases.len(), 3);
        assert_eq!(r.releases[0].files[0].size, 10);
    }

    #[test]
    fn rejects_garbage_and_unknown_schemas() {
        assert!(matches!(parse_releases("nope"), Err(AppError::Network(_))));
        assert!(matches!(
            parse_releases(r#"{"schema":2,"releases":[]}"#),
            Err(AppError::Network(_))
        ));
    }

    #[test]
    fn view_follows_language_with_english_fallback() {
        let r = parse_releases(SAMPLE).unwrap();
        let ru = to_view(&r, "ru");
        assert_eq!(ru.releases[0].title, "Вторая бета");
        assert_eq!(ru.releases[0].notes, vec!["заметка"]);
        let de = to_view(&r, "de");
        assert_eq!(de.releases[0].title, "The second beta");
    }

    #[test]
    fn view_marks_withdrawn_builds_and_skips_fileless_ones() {
        let view = to_view(&parse_releases(SAMPLE).unwrap(), "en");
        assert_eq!(view.releases.len(), 2);
        assert_eq!(view.releases[0].yanked, None);
        assert_eq!(view.releases[1].yanked.as_deref(), Some("broke saves"));
        assert_eq!(view.releases[0].sha256, "abcd");
    }

    #[test]
    fn withdrawn_reason_accepts_objects() {
        let v: Value = serde_json::from_str(r#"{"reason":{"ru":"сломано","en":"broken"}}"#).unwrap();
        assert_eq!(yanked_reason(&v, "ru").as_deref(), Some("сломано"));
        assert_eq!(yanked_reason(&v, "de").as_deref(), Some("broken"));
        assert_eq!(yanked_reason(&Value::Bool(true), "en").as_deref(), Some("withdrawn"));
        assert_eq!(yanked_reason(&Value::Null, "en"), None);
    }

    #[test]
    fn urls_must_stay_relative_to_the_site() {
        let e = Endpoint::official();
        assert_eq!(
            resolve_url(&e, "dl/0.1/NightCoop-0.1.zip").unwrap(),
            "https://nightcoop.net/dl/0.1/NightCoop-0.1.zip"
        );
        for bad in ["../x", "https://evil.example/x", "//evil.example/x", "/abs", "a/../b", "", "a b"] {
            assert!(matches!(resolve_url(&e, bad), Err(AppError::UnsafePath(_))), "{bad}");
        }
    }

    #[test]
    fn fetches_text_and_reports_http_errors() {
        let e = serve(|path| match path {
            "/releases.json" => (200, vec![], b"hello".to_vec()),
            _ => (404, vec![], b"no".to_vec()),
        });
        assert_eq!(fetch_text(&e, "releases.json").unwrap(), "hello");
        assert!(matches!(fetch_text(&e, "missing.json"), Err(AppError::Network(_))));
    }

    #[test]
    fn downloads_and_verifies_a_file() {
        let body = b"zipdata-zipdata".to_vec();
        let expected = sha(&body);
        let size = body.len() as u64;
        let e = serve(move |_| (200, vec![], body.clone()));
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        let mut last = (0, 0);
        download(&e, "dl/a.zip", &dest, size, &expected.to_uppercase(), &mut |d, t| last = (d, t)).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), b"zipdata-zipdata");
        assert_eq!(last, (size, size));
        assert!(!dest.with_extension("part").exists());
    }

    #[test]
    fn wrong_hash_is_rejected_and_nothing_is_kept() {
        let body = b"zipdata".to_vec();
        let size = body.len() as u64;
        let e = serve(move |_| (200, vec![], body.clone()));
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        let err = download(&e, "dl/a.zip", &dest, size, &"0".repeat(64), &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, AppError::Checksum));
        assert!(!dest.exists());
        assert!(!dest.with_extension("part").exists());
    }

    #[test]
    fn short_downloads_are_rejected() {
        let body = b"abc".to_vec();
        let expected = sha(&body);
        let e = serve(move |_| (200, vec![], body.clone()));
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        let err = download(&e, "dl/a.zip", &dest, 10, &expected, &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, AppError::Checksum));
        assert!(!dest.exists());
    }

    #[test]
    fn oversized_downloads_are_stopped() {
        let body = vec![7u8; (SLACK as usize) * 2];
        let e = serve(move |_| (200, vec![], body.clone()));
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        let err = download(&e, "dl/a.zip", &dest, 10, &"0".repeat(64), &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, AppError::Network(_)));
        assert!(!dest.exists());
        assert!(!dest.with_extension("part").exists());
    }

    #[test]
    fn redirects_to_other_hosts_are_refused() {
        let e = serve(|_| (302, vec![("Location".into(), "http://example.invalid/evil.zip".into())], vec![]));
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        assert!(download(&e, "dl/a.zip", &dest, 10, &"0".repeat(64), &mut |_, _| {}).is_err());
        assert!(!dest.exists());
        assert!(fetch_text(&e, "releases.json").is_err());
    }
}
