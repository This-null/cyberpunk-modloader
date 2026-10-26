use crate::error::AppError;
use reqwest::blocking::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

const MAX_LINES: usize = 40;
const MAX_CHARS: usize = 600;
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
        .get(ENDPOINT)
        .query(&[("client", "gtx"), ("sl", "auto"), ("tl", code), ("dt", "t"), ("q", text)])
        .send()
        .map_err(|e| AppError::Network(e.to_string()))?;
    if !reply.status().is_success() {
        return Err(AppError::Network(format!("the translator answered {}", reply.status())));
    }
    let raw = reply.text().map_err(|e| AppError::Network(e.to_string()))?;
    parse(&raw).ok_or_else(|| AppError::Network("the translator sent an answer we could not read".into()))
}

pub fn translate_lines(cache_file: &Path, lines: &[String], lang: &str) -> Result<Vec<String>, AppError> {
    let code = target_code(lang).ok_or_else(|| AppError::BadLink("language".into()))?;
    if lines.len() > MAX_LINES || lines.iter().any(|l| l.chars().count() > MAX_CHARS) {
        return Err(AppError::BadLink("translation".into()));
    }
    let mut cache: BTreeMap<String, String> = fs::read_to_string(cache_file)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let mut client: Option<Client> = None;
    let mut out = Vec::with_capacity(lines.len());
    let mut changed = false;
    for line in lines {
        let key = cache_key(code, line);
        if line.trim().is_empty() {
            out.push(line.clone());
        } else if let Some(hit) = cache.get(&key) {
            out.push(hit.clone());
        } else {
            let c = match &client {
                Some(c) => c,
                None => client.insert(
                    Client::builder()
                        .user_agent(format!("{}/{}", crate::nexus::APP_NAME, env!("CARGO_PKG_VERSION")))
                        .connect_timeout(Duration::from_secs(6))
                        .timeout(Duration::from_secs(10))
                        .https_only(true)
                        .build()
                        .map_err(|e| AppError::Network(e.to_string()))?,
                ),
            };
            let text = ask(c, line, code)?;
            cache.insert(key, text.clone());
            changed = true;
            out.push(text);
        }
    }
    if changed {
        if cache.len() > 2000 {
            let drop: Vec<String> = cache.keys().take(cache.len() - 2000).cloned().collect();
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
    Ok(out)
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
    fn oversized_or_unknown_requests_are_refused_before_sending_anything() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("t.json");
        let many: Vec<String> = (0..41).map(|i| format!("line {i}")).collect();
        assert!(translate_lines(&file, &many, "tr").is_err());
        assert!(translate_lines(&file, &["x".repeat(601)], "tr").is_err());
        assert!(translate_lines(&file, &["hi".into()], "klingon").is_err());
        assert!(!file.exists());
    }
}
