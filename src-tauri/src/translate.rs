use crate::error::AppError;
use reqwest::blocking::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

const MAX_LINES: usize = 60;
const MAX_CHARS: usize = 1500;
const BATCH_CHARS: usize = 4500;
const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/single";

pub fn target_code(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "en" => "en",
        "tr" => "tr",
        "ru" => "ru",
        "es" => "es",
        "pt-BR" => "pt",
        "fr" => "fr",
        "pl" => "pl",
        "de" => "de",
        _ => return None,
    })
}

fn cache_key(lang: &str, text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let hex: String = digest.iter().take(12).map(|b| format!("{b:02x}")).collect();
    format!("{lang}|{hex}")
}

pub fn parse(raw: &str) -> Option<String> {
    let body: Value = serde_json::from_str(raw).ok()?;
    let mut out = String::new();
    for seg in body.get(0)?.as_array()? {
        if let Some(s) = seg.get(0).and_then(|v| v.as_str()) {
            out.push_str(s);
        }
    }
    let out = out.trim().to_string();
    (!out.is_empty()).then_some(out)
}

fn ask(client: &Client, text: &str, code: &str) -> Result<String, AppError> {
    let reply = client
        .post(ENDPOINT)
        .query(&[("client", "gtx"), ("sl", "auto"), ("tl", code), ("dt", "t")])
        .form(&[("q", text)])
        .send()
        .map_err(|e| AppError::Network(e.to_string()))?;
    if !reply.status().is_success() {
        return Err(AppError::Network(format!("the translator answered {}", reply.status())));
    }
    let raw = reply.text().map_err(|e| AppError::Network(e.to_string()))?;
    parse(&raw).ok_or_else(|| AppError::Network("the translator sent an answer we could not read".into()))
}

fn flat(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn translate_batch(lines: &[String], ask_once: &mut dyn FnMut(&str) -> Result<String, AppError>) -> Result<Vec<String>, AppError> {
    let flats: Vec<String> = lines.iter().map(|l| flat(l)).collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut start = 0;
    while start < flats.len() {
        let mut end = start;
        let mut chars = 0;
        while end < flats.len() && (end == start || chars + flats[end].len() < BATCH_CHARS) {
            chars += flats[end].len() + 1;
            end += 1;
        }
        let group = &flats[start..end];
        let joined = group.join("\n");
        let answer = ask_once(&joined)?;
        let parts: Vec<String> = answer.split('\n').map(|p| p.trim().to_string()).collect();
        if parts.len() == group.len() && parts.iter().all(|p| !p.is_empty()) {
            out.extend(parts);
        } else {
            for line in group {
                out.push(ask_once(line)?);
            }
        }
        start = end;
    }
    Ok(out)
}

pub fn translate_via(
    cache_file: &Path,
    lines: &[String],
    lang: &str,
    ask_once: &mut dyn FnMut(&str, &str) -> Result<String, AppError>,
) -> Result<Vec<String>, AppError> {
    let code = target_code(lang).ok_or_else(|| AppError::BadLink("language".into()))?;
    if lines.len() > MAX_LINES || lines.iter().any(|l| l.chars().count() > MAX_CHARS) {
        return Err(AppError::BadLink("translation".into()));
    }
    let mut cache: BTreeMap<String, String> = fs::read_to_string(cache_file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let mut missing: Vec<String> = Vec::new();
    for line in lines {
        if !line.trim().is_empty() && !cache.contains_key(&cache_key(code, line)) && !missing.contains(line) {
            missing.push(line.clone());
        }
    }
    if !missing.is_empty() {
        let done = translate_batch(&missing, &mut |text| ask_once(text, code))?;
        for (line, translated) in missing.iter().zip(done) {
            cache.insert(cache_key(code, line), translated);
        }
        if cache.len() > 4000 {
            let drop: Vec<String> = cache.keys().take(cache.len() - 4000).cloned().collect();
            for k in drop {
                cache.remove(&k);
            }
        }
        if let Ok(text) = serde_json::to_string(&cache) {
            let tmp = cache_file.with_extension("tmp");
            if fs::write(&tmp, text).is_ok() {
                let _ = fs::rename(&tmp, cache_file);
            }
        }
    }
    Ok(lines
        .iter()
        .map(|l| if l.trim().is_empty() { l.clone() } else { cache.get(&cache_key(code, l)).cloned().unwrap_or_else(|| l.clone()) })
        .collect())
}

pub fn translate_lines(cache_file: &Path, lines: &[String], lang: &str) -> Result<Vec<String>, AppError> {
    let client = Client::builder()
        .user_agent(format!("{}/{}", crate::nexus::APP_NAME, env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(6))
        .timeout(Duration::from_secs(15))
        .https_only(true)
        .build()
        .map_err(|e| AppError::Network(e.to_string()))?;
    translate_via(cache_file, lines, lang, &mut |text, code| ask(&client, text, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_map_to_translator_codes_and_unknown_ones_are_refused() {
        assert_eq!(target_code("pt-BR"), Some("pt"));
        assert_eq!(target_code("tr"), Some("tr"));
        assert_eq!(target_code("xx"), None);
        assert_eq!(target_code(""), None);
    }

    #[test]
    fn the_translator_answer_is_joined_from_its_segments() {
        let raw = r#"[[["Merhaba ","Hello ",null,null,10],["dünya","world",null,null,10]],null,"en"]"#;
        assert_eq!(parse(raw).as_deref(), Some("Merhaba dünya"));
        assert_eq!(parse("[]"), None);
        assert_eq!(parse("not json"), None);
        assert_eq!(parse(r#"[[["  ","x"]]]"#), None);
    }

    #[test]
    fn cached_lines_never_touch_the_network() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("t.json");
        let mut cache = BTreeMap::new();
        cache.insert(cache_key("tr", "Fixed crash"), "Çökme düzeltildi".to_string());
        fs::write(&file, serde_json::to_string(&cache).unwrap()).unwrap();
        let out = translate_lines(&file, &["Fixed crash".into(), "  ".into()], "tr").unwrap();
        assert_eq!(out, vec!["Çökme düzeltildi".to_string(), "  ".to_string()]);
    }

    #[test]
    fn many_lines_go_out_in_one_request_and_come_back_in_order() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("t.json");
        let mut calls = Vec::new();
        let lines: Vec<String> = vec!["one".into(), "two\nwords".into(), "three".into()];
        let out = translate_via(&file, &lines, "tr", &mut |text, code| {
            calls.push(text.to_string());
            assert_eq!(code, "tr");
            Ok(text.split('\n').map(|l| l.to_uppercase()).collect::<Vec<_>>().join("\n"))
        })
        .unwrap();
        assert_eq!(calls, vec!["one\ntwo words\nthree".to_string()]);
        assert_eq!(out, vec!["ONE", "TWO WORDS", "THREE"]);
        let mut again = 0;
        let cached = translate_via(&file, &lines, "tr", &mut |_, _| {
            again += 1;
            Ok(String::new())
        })
        .unwrap();
        assert_eq!((again, cached), (0, out));
    }

    #[test]
    fn a_batch_that_comes_back_mangled_is_redone_line_by_line() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("t.json");
        let mut n = 0;
        let out = translate_via(&file, &["a".into(), "b".into()], "de", &mut |text, _| {
            n += 1;
            Ok(if text.contains('\n') { "all in one line".into() } else { format!("<{text}>") })
        })
        .unwrap();
        assert_eq!(out, vec!["<a>", "<b>"]);
        assert_eq!(n, 3);
    }

    #[test]
    fn repeated_lines_are_asked_once() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("t.json");
        let mut seen = Vec::new();
        let out = translate_via(&file, &["same".into(), "same".into(), "  ".into()], "fr", &mut |text, _| {
            seen.push(text.to_string());
            Ok("pareil".into())
        })
        .unwrap();
        assert_eq!(seen, vec!["same".to_string()]);
        assert_eq!(out, vec!["pareil", "pareil", "  "]);
    }

    #[test]
    fn oversized_or_unknown_requests_are_refused_before_sending_anything() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("t.json");
        let many: Vec<String> = (0..61).map(|i| format!("line {i}")).collect();
        assert!(translate_lines(&file, &many, "tr").is_err());
        assert!(translate_lines(&file, &["x".repeat(1501)], "tr").is_err());
        assert!(translate_lines(&file, &["hi".into()], "klingon").is_err());
        assert!(!file.exists());
    }
}
