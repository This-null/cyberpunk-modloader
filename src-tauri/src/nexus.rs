use crate::error::AppError;
use crate::library::ModRecord;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::HeaderValue;
use reqwest::redirect::Policy;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

pub const API_BASE: &str = "https://api.nexusmods.com/";
pub const GAME: &str = "cyberpunk2077";
pub const APP_NAME: &str = "CP2077-Mod-Launcher";
pub const CHECK_TTL_SECS: u64 = 6 * 60 * 60;
const MAX_BODY: u64 = 1024 * 1024;

#[derive(Clone)]
pub struct Api {
    base: String,
    host: String,
    allow_http: bool,
}

impl Api {
    pub fn official() -> Api {
        Api {
            base: API_BASE.to_string(),
            host: "api.nexusmods.com".to_string(),
            allow_http: false,
        }
    }

    #[cfg(test)]
    pub fn local(port: u16) -> Api {
        Api {
            base: format!("http://127.0.0.1:{port}/"),
            host: "127.0.0.1".to_string(),
            allow_http: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Validated {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub is_premium: bool,
    #[serde(default)]
    pub is_supporter: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModInfo {
    #[serde(default)]
    pub mod_id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub picture_url: Option<String>,
    #[serde(default)]
    pub updated_timestamp: u64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub category_id: u64,
    #[serde(default)]
    pub contains_adult_content: bool,
    #[serde(default)]
    pub endorsement_count: u64,
    #[serde(default)]
    pub mod_downloads: u64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheEntry {
    pub checked_at: u64,
    pub info: ModInfo,
}

pub type Cache = BTreeMap<u64, CacheEntry>;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Status {
    pub name: String,
    pub mod_id: u64,
    pub installed: String,
    pub latest: Option<String>,
    pub update: bool,
    pub checked_at: Option<u64>,
    pub page_url: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NexusFile {
    pub name: String,
    pub mod_id: u64,
    pub version: String,
}

fn net(e: impl Display) -> AppError {
    AppError::Network(e.to_string())
}

pub fn valid_key(key: &str) -> bool {
    (16..=256).contains(&key.len())
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_=+/.".contains(c))
}

fn client(api: &Api) -> Result<Client, AppError> {
    let host = api.host.clone();
    let allow_http = api.allow_http;
    Client::builder()
        .user_agent(format!("{APP_NAME}/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .https_only(!allow_http)
        .redirect(Policy::custom(move |attempt| {
            let same_host = attempt.url().host_str() == Some(host.as_str());
            if attempt.previous().len() >= 3 {
                attempt.error("too many redirects")
            } else if same_host && (attempt.url().scheme() == "https" || allow_http) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(net)
}

fn get<T: DeserializeOwned>(api: &Api, key: &str, path: &str) -> Result<T, AppError> {
    get_ex(api, key, path, false)
}

fn get_ex<T: DeserializeOwned>(api: &Api, key: &str, path: &str, forbidden_means_premium: bool) -> Result<T, AppError> {
    if !valid_key(key) {
        return Err(AppError::NexusKey);
    }
    let mut secret = HeaderValue::from_str(key).map_err(|_| AppError::NexusKey)?;
    secret.set_sensitive(true);
    let response = client(api)?
        .get(format!("{}{}", api.base, path))
        .header("APIKEY", secret)
        .header("Application-Name", APP_NAME)
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .send()
        .map_err(net)?;
    match response.status() {
        s if s.is_success() => {}
        StatusCode::FORBIDDEN if forbidden_means_premium => return Err(AppError::NexusPremium),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(AppError::NexusKey),
        StatusCode::NOT_FOUND => return Err(AppError::NotFound("nexus mod".into())),
        StatusCode::TOO_MANY_REQUESTS => return Err(AppError::RateLimited),
        s => return Err(net(format!("Nexus answered {s}"))),
    }
    let mut buf = Vec::new();
    response.take(MAX_BODY + 1).read_to_end(&mut buf).map_err(net)?;
    if buf.len() as u64 > MAX_BODY {
        return Err(net("the answer is too large"));
    }
    serde_json::from_slice(&buf).map_err(|_| net("Nexus sent an answer we could not read"))
}

pub fn validate(api: &Api, key: &str) -> Result<Validated, AppError> {
    get(api, key, "v1/users/validate.json")
}

pub fn mod_info(api: &Api, key: &str, mod_id: u64) -> Result<ModInfo, AppError> {
    get(api, key, &format!("v1/games/{GAME}/mods/{mod_id}.json"))
}

pub fn page_url(mod_id: u64) -> String {
    format!("https://www.nexusmods.com/{GAME}/mods/{mod_id}")
}

pub fn parse_filename(file_name: &str) -> Option<NexusFile> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"^(?P<name>.+?)-(?P<id>\d{1,9})-(?P<ver>\d+(?:-\d+)*)-(?P<ts>\d{9,11})(?:\.[A-Za-z0-9]{1,5})?$")
            .expect("valid pattern")
    });
    let c = re.captures(file_name.trim())?;
    Some(NexusFile {
        name: c["name"].to_string(),
        mod_id: c["id"].parse().ok()?,
        version: c["ver"].replace('-', "."),
    })
}

pub fn parse_target(input: &str) -> Option<u64> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let text = input.trim();
    if !text.is_empty() && text.len() <= 9 && text.chars().all(|c| c.is_ascii_digit()) {
        return text.parse().ok().filter(|id| *id > 0);
    }
    let re = RE.get_or_init(|| {
        Regex::new(r"(?i)^https?://(?:www\.)?nexusmods\.com/cyberpunk2077/mods/(\d{1,9})(?:[/?#].*)?$").expect("valid pattern")
    });
    re.captures(text)?.get(1)?.as_str().parse().ok().filter(|id| *id > 0)
}

fn numbers(version: &str) -> Option<(Vec<u64>, String)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^\s*[vV]?(\d+(?:\.\d+)*)(.*)$").expect("valid pattern"));
    let c = re.captures(version)?;
    let parts = c[1].split('.').map(|p| p.parse::<u64>().ok()).collect::<Option<Vec<_>>>()?;
    Some((parts, c[2].trim().to_string()))
}

fn is_pre(suffix: &str) -> bool {
    let lower = suffix.to_ascii_lowercase();
    ["alpha", "beta", "rc", "pre", "dev", "snapshot"].iter().any(|k| lower.contains(k))
}

pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let (mut x, sx) = numbers(a)?;
    let (mut y, sy) = numbers(b)?;
    let len = x.len().max(y.len());
    x.resize(len, 0);
    y.resize(len, 0);
    match x.cmp(&y) {
        Ordering::Equal if sx == sy => Some(Ordering::Equal),
        Ordering::Equal if sx.is_empty() && is_pre(&sy) => Some(Ordering::Greater),
        Ordering::Equal if sy.is_empty() && is_pre(&sx) => Some(Ordering::Less),
        Ordering::Equal => None,
        other => Some(other),
    }
}

pub fn update_available(installed: &str, latest: &str) -> bool {
    compare_versions(installed, latest) == Some(Ordering::Less)
}

pub fn needs_refresh(entry: Option<&CacheEntry>, now: u64, ttl: u64) -> bool {
    entry.is_none_or(|e| now.saturating_sub(e.checked_at) >= ttl || e.checked_at > now)
}

pub fn check_all(
    api: &Api,
    key: &str,
    records: &[ModRecord],
    cache: &mut Cache,
    now: u64,
    ttl: u64,
) -> Result<Vec<Status>, AppError> {
    let mut stop = false;
    let mut failures: BTreeMap<u64, String> = BTreeMap::new();
    for rec in records {
        let Some(id) = rec.nexus_id else { continue };
        if !needs_refresh(cache.get(&id), now, ttl) || failures.contains_key(&id) {
            continue;
        }
        if stop {
            failures.insert(id, "rate_limited".into());
            continue;
        }
        match mod_info(api, key, id) {
            Ok(info) => {
                cache.insert(id, CacheEntry { checked_at: now, info });
            }
            Err(AppError::NexusKey) => return Err(AppError::NexusKey),
            Err(AppError::RateLimited) => {
                stop = true;
                failures.insert(id, "rate_limited".into());
            }
            Err(AppError::NotFound(_)) => {
                failures.insert(id, "not_found".into());
            }
            Err(_) => {
                failures.insert(id, "network".into());
            }
        }
    }
    Ok(records
        .iter()
        .filter_map(|rec| {
            let id = rec.nexus_id?;
            let entry = cache.get(&id);
            let latest = entry.map(|e| e.info.version.clone()).filter(|v| !v.is_empty());
            let update = latest
                .as_deref()
                .is_some_and(|l| !rec.version.is_empty() && update_available(&rec.version, l));
            Some(Status {
                name: rec.name.clone(),
                mod_id: id,
                installed: rec.version.clone(),
                latest,
                update,
                checked_at: entry.map(|e| e.checked_at),
                page_url: page_url(id),
                error: failures.get(&id).cloned(),
            })
        })
        .collect())
}

pub const LIST_TTL_SECS: u64 = 10 * 60;
pub const CATEGORY_TTL_SECS: u64 = 24 * 60 * 60;
pub const LIST_KINDS: [&str; 3] = ["trending", "latest_added", "latest_updated"];
const MAX_DOWNLOAD: u64 = 1024 * 1024 * 1024;
const DOWNLOAD_SLACK: u64 = 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Category {
    pub category_id: u64,
    pub name: String,
    pub parent: Option<u64>,
}

#[derive(Deserialize)]
struct RawCategory {
    #[serde(default)]
    category_id: u64,
    #[serde(default)]
    name: String,
    #[serde(default)]
    parent_category: serde_json::Value,
}

#[derive(Deserialize)]
struct RawGame {
    #[serde(default)]
    categories: Vec<RawCategory>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    #[serde(default)]
    pub file_id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub category_name: String,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub size_kb: u64,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub uploaded_timestamp: u64,
}

#[derive(Deserialize)]
struct FilesReply {
    #[serde(default)]
    files: Vec<FileInfo>,
}

#[derive(Deserialize)]
struct LinkReply {
    #[serde(rename = "URI", default)]
    uri: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrowseCache {
    #[serde(default)]
    pub lists: BTreeMap<String, (u64, Vec<ModInfo>)>,
    #[serde(default)]
    pub categories: Option<(u64, Vec<Category>)>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BrowseData {
    pub items: Vec<ModInfo>,
    pub categories: Vec<Category>,
    pub fetched_at: u64,
}

pub fn is_fresh(fetched_at: u64, now: u64, ttl: u64) -> bool {
    now >= fetched_at && now - fetched_at < ttl
}

pub fn game_categories(api: &Api, key: &str) -> Result<Vec<Category>, AppError> {
    let game: RawGame = get(api, key, &format!("v1/games/{GAME}.json"))?;
    Ok(game
        .categories
        .into_iter()
        .map(|c| Category {
            category_id: c.category_id,
            name: c.name,
            parent: c.parent_category.as_u64(),
        })
        .collect())
}

pub fn list(api: &Api, key: &str, kind: &str) -> Result<Vec<ModInfo>, AppError> {
    if !LIST_KINDS.contains(&kind) {
        return Err(AppError::NotFound(kind.to_string()));
    }
    get(api, key, &format!("v1/games/{GAME}/mods/{kind}.json"))
}

pub fn files(api: &Api, key: &str, mod_id: u64) -> Result<Vec<FileInfo>, AppError> {
    let reply: FilesReply = get(api, key, &format!("v1/games/{GAME}/mods/{mod_id}/files.json"))?;
    Ok(reply.files)
}

pub fn choose_file(files: &[FileInfo]) -> Option<&FileInfo> {
    let usable = |f: &&FileInfo| !matches!(f.category_name.to_ascii_uppercase().as_str(), "OLD_VERSION" | "ARCHIVED" | "DELETED");
    files
        .iter()
        .filter(usable)
        .find(|f| f.is_primary)
        .or_else(|| files.iter().filter(usable).find(|f| f.category_name.eq_ignore_ascii_case("MAIN")))
        .or_else(|| files.iter().filter(usable).max_by_key(|f| f.uploaded_timestamp))
}

pub fn download_link(api: &Api, key: &str, mod_id: u64, file_id: u64) -> Result<String, AppError> {
    download_link_with(api, key, mod_id, file_id, None)
}

pub fn download_link_with(
    api: &Api,
    key: &str,
    mod_id: u64,
    file_id: u64,
    link: Option<(&str, u64)>,
) -> Result<String, AppError> {
    let mut path = format!("v1/games/{GAME}/mods/{mod_id}/files/{file_id}/download_link.json");
    if let Some((k, expires)) = link {
        if k.is_empty() || !k.chars().all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)) {
            return Err(AppError::BadLink("nxm".into()));
        }
        path.push_str(&format!("?key={k}&expires={expires}"));
    }
    let links: Vec<LinkReply> = match get_ex(api, key, &path, link.is_none()) {
        Err(AppError::NexusKey) if link.is_some() => return Err(AppError::NexusLinkExpired),
        other => other?,
    };
    links
        .into_iter()
        .map(|l| l.uri)
        .find(|u| !u.is_empty())
        .ok_or_else(|| AppError::NotFound("download link".into()))
}

pub fn allowed_download_host(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    if url.len() > 2000 || url.chars().any(|c| c.is_control() || c.is_whitespace() || c == '@' || c == '\\') {
        return false;
    }
    let host = rest.split(['/', '?', '#', ':']).next().unwrap_or("").to_ascii_lowercase();
    ["nexusmods.com", "nexus-cdn.com"]
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

pub fn download_to(
    url: &str,
    dest: &std::path::Path,
    expected_size: u64,
    trusted: std::sync::Arc<dyn Fn(&str) -> bool + Send + Sync>,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(), AppError> {
    if !trusted(url) {
        return Err(AppError::UntrustedHost(url.chars().take(80).collect()));
    }
    let limit = if expected_size == 0 { MAX_DOWNLOAD } else { expected_size + DOWNLOAD_SLACK };
    let check = trusted.clone();
    let client = Client::builder()
        .user_agent(format!("{APP_NAME}/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(600))
        .redirect(Policy::custom(move |attempt| {
            if attempt.previous().len() >= 4 {
                attempt.error("too many redirects")
            } else if check(attempt.url().as_str()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(net)?;
    let mut response = client.get(url).send().map_err(net)?;
    if !response.status().is_success() {
        let host = response.url().host_str().unwrap_or("").to_string();
        if response.status().is_redirection() {
            return Err(AppError::UntrustedHost(host));
        }
        return Err(net(format!("the download server answered {}", response.status())));
    }
    if response.content_length().is_some_and(|len| len > limit) {
        return Err(net("the download is larger than announced"));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let part = dest.with_extension("part");
    let result = (|| -> Result<(), AppError> {
        use std::io::Write;
        let mut out = std::fs::File::create(&part)?;
        let mut done = 0u64;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = response.read(&mut buf).map_err(net)?;
            if n == 0 {
                break;
            }
            done += n as u64;
            if done > limit {
                return Err(net("the download is larger than announced"));
            }
            out.write_all(&buf[..n])?;
            progress(done, expected_size);
        }
        out.flush()?;
        if done == 0 {
            return Err(net("the download was empty"));
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            std::fs::rename(&part, dest)?;
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

pub fn browse(
    api: &Api,
    key: &str,
    kind: &str,
    cache: &mut BrowseCache,
    now: u64,
) -> Result<BrowseData, AppError> {
    let cached = cache.lists.get(kind).cloned();
    let (fetched_at, items) = match cached {
        Some((at, items)) if is_fresh(at, now, LIST_TTL_SECS) => (at, items),
        stale => match list(api, key, kind) {
            Ok(items) => {
                cache.lists.insert(kind.to_string(), (now, items.clone()));
                (now, items)
            }
            Err(AppError::RateLimited) | Err(AppError::Network(_)) if stale.is_some() => stale.unwrap_or_default(),
            Err(e) => return Err(e),
        },
    };
    let categories = match cache.categories.clone() {
        Some((at, cats)) if is_fresh(at, now, CATEGORY_TTL_SECS) => cats,
        stale => match game_categories(api, key) {
            Ok(cats) => {
                cache.categories = Some((now, cats.clone()));
                cats
            }
            Err(AppError::NexusKey) => return Err(AppError::NexusKey),
            Err(_) => stale.map(|(_, c)| c).unwrap_or_default(),
        },
    };
    Ok(BrowseData { items, categories, fetched_at })
}

const INTROSPECTION: &str = r#"{"query":"query { __schema { queryType { name } types { name kind inputFields { name type { name kind ofType { name kind ofType { name kind } } } } fields { name args { name type { name kind ofType { name kind ofType { name kind } } } } type { name kind ofType { name kind ofType { name kind } } } } enumValues { name } } } }"}"#;

pub fn probe_schema(api: &Api, key: &str, dest: &std::path::Path) -> Result<u64, AppError> {
    if !valid_key(key) {
        return Err(AppError::NexusKey);
    }
    let mut secret = HeaderValue::from_str(key).map_err(|_| AppError::NexusKey)?;
    secret.set_sensitive(true);
    let response = client(api)?
        .post(format!("{}v2/graphql", api.base))
        .header("APIKEY", secret)
        .header("Application-Name", APP_NAME)
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .header("Content-Type", "application/json")
        .body(INTROSPECTION)
        .send()
        .map_err(net)?;
    match response.status() {
        s if s.is_success() => {}
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(AppError::NexusKey),
        s => return Err(net(format!("Nexus answered {s}"))),
    }
    let mut buf = Vec::new();
    response.take(8 * 1024 * 1024 + 1).read_to_end(&mut buf).map_err(net)?;
    if buf.len() as u64 > 8 * 1024 * 1024 {
        return Err(net("the answer is too large"));
    }
    std::fs::write(dest, &buf)?;
    Ok(buf.len() as u64)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SearchParams {
    pub query: String,
    pub categories: Vec<String>,
    pub hide_adult: bool,
    pub sort: String,
    pub tag: Option<String>,
    pub offset: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct SearchPage {
    pub items: Vec<ModInfo>,
    pub total: u64,
}

const SEARCH_QUERY: &str = "query($filter: ModsFilter, $sort: [ModsSort!], $offset: Int, $count: Int) { mods(filter: $filter, sort: $sort, offset: $offset, count: $count) { totalCount nodes { modId name summary version author pictureUrl thumbnailUrl downloads endorsements adultContent status modCategory { categoryId } tags { name } } } }";

fn eq(value: &str) -> serde_json::Value {
    serde_json::json!([{ "value": value, "op": "EQUALS" }])
}

pub fn clean_term(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control() && !"*?\\\"".contains(*c))
        .take(80)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn check_search(p: &SearchParams) -> Result<(), AppError> {
    let sorts = ["downloads", "endorsements", "createdAt", "updatedAt", "relevance"];
    let ok = sorts.contains(&p.sort.as_str())
        && p.count >= 1
        && p.count <= 48
        && p.offset <= 10_000
        && p.categories.len() <= 40
        && p.categories.iter().all(|c| !c.is_empty() && c.chars().count() <= 80)
        && p.tag.as_ref().is_none_or(|t| !t.is_empty() && t.chars().count() <= 80);
    if ok {
        Ok(())
    } else {
        Err(AppError::BadLink("search".into()))
    }
}

pub fn search_body(p: &SearchParams) -> serde_json::Value {
    use serde_json::json;
    let mut filter = json!({
        "op": "AND",
        "gameDomainName": eq(GAME),
        "status": eq("published"),
    });
    if p.hide_adult {
        filter["adultContent"] = json!([{ "value": false, "op": "EQUALS" }]);
    }
    let term = clean_term(&p.query);
    if !term.is_empty() {
        filter["nameStemmed"] = json!([{ "value": term, "op": "MATCHES" }]);
    }
    if let Some(tag) = &p.tag {
        filter["tag"] = eq(tag);
    }
    match p.categories.len() {
        0 => {}
        1 => filter["categoryName"] = eq(&p.categories[0]),
        _ => {
            let any: Vec<_> = p.categories.iter().map(|c| json!({ "categoryName": eq(c) })).collect();
            filter["filter"] = json!([{ "op": "OR", "filter": any }]);
        }
    }
    json!({
        "query": SEARCH_QUERY,
        "variables": {
            "filter": filter,
            "sort": [{ p.sort.as_str(): { "direction": "DESC" } }],
            "offset": p.offset,
            "count": p.count,
        }
    })
}

pub fn parse_search(v: &serde_json::Value) -> Result<SearchPage, AppError> {
    if let Some(errors) = v.get("errors").and_then(|e| e.as_array()).filter(|e| !e.is_empty()) {
        let msg = errors[0].get("message").and_then(|m| m.as_str()).unwrap_or("unknown error");
        return Err(net(format!("Nexus search failed: {}", msg.chars().take(160).collect::<String>())));
    }
    let page = v.pointer("/data/mods").ok_or_else(|| net("Nexus search gave no data"))?;
    let nodes = page.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();
    let text = |n: &serde_json::Value, k: &str| n.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let number = |n: &serde_json::Value, k: &str| n.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
    let items = nodes
        .iter()
        .filter_map(|n| {
            let mod_id = n.get("modId").and_then(|x| x.as_u64())?;
            Some(ModInfo {
                mod_id,
                name: text(n, "name"),
                summary: text(n, "summary"),
                version: text(n, "version").unwrap_or_default(),
                picture_url: text(n, "pictureUrl").or_else(|| text(n, "thumbnailUrl")),
                updated_timestamp: 0,
                status: text(n, "status").unwrap_or_default(),
                author: text(n, "author").unwrap_or_default(),
                category_id: n.pointer("/modCategory/categoryId").and_then(|x| x.as_u64()).unwrap_or(0),
                contains_adult_content: n.get("adultContent").and_then(|x| x.as_bool()).unwrap_or(false),
                endorsement_count: number(n, "endorsements"),
                mod_downloads: number(n, "downloads"),
                tags: n
                    .get("tags")
                    .and_then(|t| t.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|t| t.get("name").and_then(|x| x.as_str()).map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect();
    Ok(SearchPage { items, total: page.get("totalCount").and_then(|x| x.as_u64()).unwrap_or(0) })
}

pub fn search(api: &Api, key: &str, params: &SearchParams) -> Result<SearchPage, AppError> {
    if !valid_key(key) {
        return Err(AppError::NexusKey);
    }
    check_search(params)?;
    let mut secret = HeaderValue::from_str(key).map_err(|_| AppError::NexusKey)?;
    secret.set_sensitive(true);
    let response = client(api)?
        .post(format!("{}v2/graphql", api.base))
        .header("APIKEY", secret)
        .header("Application-Name", APP_NAME)
        .header("Application-Version", env!("CARGO_PKG_VERSION"))
        .header("Content-Type", "application/json")
        .body(search_body(params).to_string())
        .send()
        .map_err(net)?;
    match response.status() {
        s if s.is_success() => {}
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(AppError::NexusKey),
        s => return Err(net(format!("Nexus answered {s}"))),
    }
    let mut buf = Vec::new();
    response.take(4 * 1024 * 1024 + 1).read_to_end(&mut buf).map_err(net)?;
    if buf.len() as u64 > 4 * 1024 * 1024 {
        return Err(net("the answer is too large"));
    }
    let value: serde_json::Value = serde_json::from_slice(&buf).map_err(net)?;
    parse_search(&value)
}

pub trait KeyStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, AppError>;
    fn set(&self, key: &str) -> Result<(), AppError>;
    fn delete(&self) -> Result<(), AppError>;
}

pub struct CredStore;

impl CredStore {
    fn entry() -> Result<keyring::Entry, AppError> {
        keyring::Entry::new("CP2077 Mod Launcher", "nexus-api-key")
            .map_err(|e| AppError::Io(format!("credential store: {e}")))
    }
}

impl KeyStore for CredStore {
    fn get(&self) -> Result<Option<String>, AppError> {
        match Self::entry()?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AppError::Io(format!("credential store: {e}"))),
        }
    }

    fn set(&self, key: &str) -> Result<(), AppError> {
        Self::entry()?
            .set_password(key)
            .map_err(|e| AppError::Io(format!("credential store: {e}")))
    }

    fn delete(&self) -> Result<(), AppError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Io(format!("credential store: {e}"))),
        }
    }
}

#[cfg(test)]
pub struct MemStore(pub std::sync::Mutex<Option<String>>);

#[cfg(test)]
impl KeyStore for MemStore {
    fn get(&self) -> Result<Option<String>, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn set(&self, key: &str) -> Result<(), AppError> {
        *self.0.lock().unwrap() = Some(key.to_string());
        Ok(())
    }
    fn delete(&self) -> Result<(), AppError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader, Read as _, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    const KEY: &str = "abcDEF0123456789abcDEF0123456789==";

    type Headers = HashMap<String, String>;
    type Reply = (u16, Vec<(String, String)>, String);

    fn serve(handler: impl Fn(&str, &Headers) -> Reply + Send + 'static) -> Api {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let mut headers = Headers::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some((k, v)) = line.trim_end().split_once(':') {
                        headers.insert(k.to_ascii_lowercase(), v.trim().to_string());
                    }
                }
                if let Some(len) = headers.get("content-length").and_then(|v| v.parse::<usize>().ok()) {
                    let mut skipped = vec![0u8; len];
                    reader.read_exact(&mut skipped).unwrap();
                }
                let path = first.split_whitespace().nth(1).unwrap_or("/").to_string();
                let (status, extra, body) = handler(&path, &headers);
                let mut head = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n",
                    body.len()
                );
                for (k, v) in extra {
                    head.push_str(&format!("{k}: {v}\r\n"));
                }
                head.push_str("\r\n");
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body.as_bytes());
            }
        });
        Api::local(port)
    }

    fn rec(name: &str, version: &str, id: Option<u64>) -> ModRecord {
        ModRecord {
            name: name.into(),
            version: version.into(),
            nexus_id: id,
            ..ModRecord::default()
        }
    }

    #[test]
    fn reads_nexus_file_names() {
        let f = parse_filename("Cyber Engine Tweaks-107-1-16-3-1760000000.zip").unwrap();
        assert_eq!((f.name.as_str(), f.mod_id, f.version.as_str()), ("Cyber Engine Tweaks", 107, "1.16.3"));
        let g = parse_filename("Foo-2077-Fix-12-1-0-1700000000").unwrap();
        assert_eq!((g.name.as_str(), g.mod_id, g.version.as_str()), ("Foo-2077-Fix", 12, "1.0"));
        let h = parse_filename("Short-5-2-1700000000.7z").unwrap();
        assert_eq!((h.mod_id, h.version.as_str()), (5, "2"));
    }

    #[test]
    fn ignores_names_that_are_not_from_nexus() {
        assert_eq!(parse_filename("my-mod.zip"), None);
        assert_eq!(parse_filename("Cool-1.2.3.zip"), None);
        assert_eq!(parse_filename("Name-107-1-0-123.zip"), None);
        assert_eq!(parse_filename(""), None);
    }

    #[test]
    fn accepts_a_mod_number_or_a_cyberpunk_page_link_only() {
        assert_eq!(parse_target("107"), Some(107));
        assert_eq!(parse_target("  107 "), Some(107));
        assert_eq!(parse_target("https://www.nexusmods.com/cyberpunk2077/mods/107"), Some(107));
        assert_eq!(parse_target("https://nexusmods.com/cyberpunk2077/mods/107?tab=files"), Some(107));
        assert_eq!(parse_target("HTTP://WWW.NEXUSMODS.COM/cyberpunk2077/mods/8#top"), Some(8));
        for bad in [
            "",
            "0",
            "abc",
            "-5",
            "1234567890",
            "https://www.nexusmods.com/skyrim/mods/107",
            "https://evil.example/cyberpunk2077/mods/107",
            "https://www.nexusmods.com.evil.example/cyberpunk2077/mods/107",
            "https://www.nexusmods.com/cyberpunk2077/mods/",
            "ftp://www.nexusmods.com/cyberpunk2077/mods/107",
        ] {
            assert_eq!(parse_target(bad), None, "{bad}");
        }
    }

    #[test]
    fn compares_versions_by_their_numbers() {
        assert_eq!(compare_versions("1.2.3", "1.10.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("2.0", "2.0.0"), Some(Ordering::Equal));
        assert_eq!(compare_versions("v1.5", "1.4.9"), Some(Ordering::Greater));
        assert_eq!(compare_versions("1.0a", "1.0b"), None);
        assert_eq!(compare_versions("beta", "1.0"), None);
        assert!(update_available("1.0.0", "1.0.1"));
        assert!(!update_available("1.0.1", "1.0.1"));
        assert!(!update_available("2.0", "1.9"));
        assert!(!update_available("1.0a", "1.0b"));
        assert!(!update_available("", "1.0"));
    }

    #[test]
    fn a_pre_release_is_older_than_the_same_numbered_release() {
        assert_eq!(compare_versions("1.0 beta", "1.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.0", "1.0-rc1"), Some(Ordering::Greater));
        assert_eq!(compare_versions("2.1-DEV", "2.1"), Some(Ordering::Less));
        assert!(update_available("1.0 beta", "1.0"));
        assert!(!update_available("1.0", "1.0 beta"));
        assert!(!update_available("1.0a", "1.0"));
    }

    #[test]
    fn decides_when_a_cached_answer_is_too_old() {
        let entry = CacheEntry { checked_at: 1000, info: ModInfo::default() };
        assert!(needs_refresh(None, 1000, 60));
        assert!(!needs_refresh(Some(&entry), 1059, 60));
        assert!(needs_refresh(Some(&entry), 1060, 60));
        assert!(needs_refresh(Some(&entry), 500, 60));
    }

    #[test]
    fn keys_are_checked_before_any_request_is_made() {
        assert!(valid_key(KEY));
        assert!(!valid_key("short"));
        assert!(!valid_key("has space has space has space"));
        assert!(!valid_key("line\nbreak0123456789012345"));
        assert!(!valid_key(&"a".repeat(300)));
        let api = Api::official();
        assert!(matches!(validate(&api, "short"), Err(AppError::NexusKey)));
    }

    #[test]
    fn validation_sends_the_key_only_in_a_header() {
        let seen: Arc<Mutex<Vec<(String, Headers)>>> = Arc::default();
        let log = seen.clone();
        let api = serve(move |path, headers| {
            log.lock().unwrap().push((path.to_string(), headers.clone()));
            (200, vec![], r#"{"user_id":1,"key":"x","name":"Ayse","is_premium":true,"is_supporter":false,"email":"a@b.c","profile_url":"u"}"#.into())
        });
        let user = validate(&api, KEY).unwrap();
        assert_eq!(user, Validated { name: "Ayse".into(), is_premium: true, is_supporter: false });
        let seen = seen.lock().unwrap();
        let (path, headers) = &seen[0];
        assert_eq!(path, "/v1/users/validate.json");
        assert!(!path.contains(KEY));
        assert_eq!(headers.get("apikey").map(String::as_str), Some(KEY));
        assert_eq!(headers.get("application-name").map(String::as_str), Some(APP_NAME));
        assert_eq!(headers.get("application-version").map(String::as_str), Some(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn answers_map_to_clear_errors() {
        let api = serve(|path, _| match path {
            "/v1/games/cyberpunk2077/mods/1.json" => (401, vec![], r#"{"message":"no"}"#.into()),
            "/v1/games/cyberpunk2077/mods/2.json" => (404, vec![], "{}".into()),
            "/v1/games/cyberpunk2077/mods/3.json" => (429, vec![], "{}".into()),
            "/v1/games/cyberpunk2077/mods/4.json" => (500, vec![], "{}".into()),
            "/v1/games/cyberpunk2077/mods/5.json" => (200, vec![], "not json".into()),
            _ => (200, vec![], r#"{"mod_id":6,"name":"N","summary":"S","version":"1.2","picture_url":"p","updated_timestamp":9,"status":"published"}"#.into()),
        });
        assert!(matches!(mod_info(&api, KEY, 1), Err(AppError::NexusKey)));
        assert!(matches!(mod_info(&api, KEY, 2), Err(AppError::NotFound(_))));
        assert!(matches!(mod_info(&api, KEY, 3), Err(AppError::RateLimited)));
        assert!(matches!(mod_info(&api, KEY, 4), Err(AppError::Network(_))));
        assert!(matches!(mod_info(&api, KEY, 5), Err(AppError::Network(_))));
        let info = mod_info(&api, KEY, 6).unwrap();
        assert_eq!((info.version.as_str(), info.name.as_deref()), ("1.2", Some("N")));
    }

    #[test]
    fn redirects_to_other_hosts_are_not_followed() {
        let api = serve(|_, _| (302, vec![("Location".into(), "http://example.invalid/x".into())], String::new()));
        assert!(mod_info(&api, KEY, 1).is_err());
    }

    fn counting_api(hits: Arc<AtomicUsize>) -> Api {
        serve(move |path, _| {
            hits.fetch_add(1, AtomicOrdering::SeqCst);
            match path {
                "/v1/games/cyberpunk2077/mods/10.json" => (200, vec![], r#"{"mod_id":10,"version":"2.0.0"}"#.into()),
                "/v1/games/cyberpunk2077/mods/11.json" => (404, vec![], "{}".into()),
                _ => (429, vec![], "{}".into()),
            }
        })
    }

    #[test]
    fn a_check_reports_updates_and_remembers_answers() {
        let hits = Arc::new(AtomicUsize::new(0));
        let api = counting_api(hits.clone());
        let records = vec![rec("a", "1.0.0", Some(10)), rec("b", "2.0.0", Some(10)), rec("c", "1", None)];
        let mut cache = Cache::new();
        let out = check_all(&api, KEY, &records, &mut cache, 5000, CHECK_TTL_SECS).unwrap();
        assert_eq!(out.len(), 2);
        assert!(out[0].update && out[0].latest.as_deref() == Some("2.0.0"));
        assert!(!out[1].update);
        assert_eq!(out[0].checked_at, Some(5000));
        assert_eq!(out[0].page_url, "https://www.nexusmods.com/cyberpunk2077/mods/10");
        assert_eq!(hits.load(AtomicOrdering::SeqCst), 1);
        check_all(&api, KEY, &records, &mut cache, 5000 + CHECK_TTL_SECS - 1, CHECK_TTL_SECS).unwrap();
        assert_eq!(hits.load(AtomicOrdering::SeqCst), 1);
        check_all(&api, KEY, &records, &mut cache, 5000 + CHECK_TTL_SECS, CHECK_TTL_SECS).unwrap();
        assert_eq!(hits.load(AtomicOrdering::SeqCst), 2);
    }

    #[test]
    fn one_missing_mod_does_not_hide_the_others() {
        let hits = Arc::new(AtomicUsize::new(0));
        let api = counting_api(hits);
        let records = vec![rec("gone", "1", Some(11)), rec("ok", "1.0", Some(10))];
        let mut cache = Cache::new();
        let out = check_all(&api, KEY, &records, &mut cache, 1, 60).unwrap();
        assert_eq!(out[0].error.as_deref(), Some("not_found"));
        assert!(out[1].update && out[1].error.is_none());
    }

    #[test]
    fn rate_limiting_stops_further_requests_but_keeps_cached_answers() {
        let hits = Arc::new(AtomicUsize::new(0));
        let api = counting_api(hits.clone());
        let records = vec![rec("limited", "1", Some(12)), rec("also", "1", Some(13)), rec("cached", "1.0", Some(10))];
        let mut cache = Cache::new();
        cache.insert(10, CacheEntry { checked_at: 100, info: ModInfo { version: "1.5".into(), ..ModInfo::default() } });
        let out = check_all(&api, KEY, &records, &mut cache, 100, 60 * 60).unwrap();
        assert_eq!(hits.load(AtomicOrdering::SeqCst), 1);
        assert_eq!(out[0].error.as_deref(), Some("rate_limited"));
        assert_eq!(out[1].error.as_deref(), Some("rate_limited"));
        assert!(out[2].update && out[2].error.is_none());
    }

    #[test]
    fn a_rejected_key_aborts_the_whole_check() {
        let api = serve(|_, _| (401, vec![], "{}".into()));
        let records = vec![rec("a", "1", Some(10))];
        let mut cache = Cache::new();
        assert!(matches!(check_all(&api, KEY, &records, &mut cache, 1, 60), Err(AppError::NexusKey)));
    }

    #[test]
    fn categories_come_from_the_game_info() {
        let api = serve(|path, _| {
            assert_eq!(path, "/v1/games/cyberpunk2077.json");
            (200, vec![], r#"{"id":3333,"name":"Cyberpunk 2077","categories":[{"category_id":1,"name":"Visuals","parent_category":false},{"category_id":2,"name":"Weapons","parent_category":1}]}"#.into())
        });
        let cats = game_categories(&api, KEY).unwrap();
        assert_eq!(cats[0], Category { category_id: 1, name: "Visuals".into(), parent: None });
        assert_eq!(cats[1].parent, Some(1));
    }

    #[test]
    fn lists_carry_what_the_cards_need() {
        let api = serve(|path, _| {
            assert_eq!(path, "/v1/games/cyberpunk2077/mods/trending.json");
            (200, vec![], r#"[{"mod_id":7,"name":"Neon","summary":"S","version":"1.2","picture_url":"https://staticdelivery.nexusmods.com/p.png","author":"Ayse","category_id":4,"contains_adult_content":true,"endorsement_count":12,"mod_downloads":345,"updated_timestamp":9,"status":"published","extra":"ignored"}]"#.into())
        });
        let items = list(&api, KEY, "trending").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!((items[0].mod_id, items[0].author.as_str(), items[0].category_id), (7, "Ayse", 4));
        assert!(items[0].contains_adult_content);
        assert_eq!((items[0].endorsement_count, items[0].mod_downloads), (12, 345));
        assert!(matches!(list(&api, KEY, "../secrets"), Err(AppError::NotFound(_))));
    }

    #[test]
    fn the_main_file_is_picked_and_old_ones_are_skipped() {
        let f = |id: u64, cat: &str, primary: bool, at: u64| FileInfo { file_id: id, category_name: cat.into(), is_primary: primary, uploaded_timestamp: at, ..FileInfo::default() };
        assert_eq!(choose_file(&[f(1, "MAIN", false, 5), f(2, "OPTIONAL", true, 1)]).unwrap().file_id, 2);
        assert_eq!(choose_file(&[f(1, "OPTIONAL", false, 9), f(2, "MAIN", false, 1)]).unwrap().file_id, 2);
        assert_eq!(choose_file(&[f(1, "OPTIONAL", false, 3), f(2, "UPDATE", false, 8)]).unwrap().file_id, 2);
        assert_eq!(choose_file(&[f(1, "OLD_VERSION", true, 3), f(2, "ARCHIVED", false, 8)]), None);
        assert_eq!(choose_file(&[]), None);
    }

    #[test]
    fn files_and_download_links_are_read_from_the_api() {
        let api = serve(|path, _| match path {
            "/v1/games/cyberpunk2077/mods/7/files.json" => (200, vec![], r#"{"files":[{"file_id":70,"name":"Main","version":"1.2","category_name":"MAIN","is_primary":true,"size_kb":10,"file_name":"Neon-7-1-2-1700000000.zip","uploaded_timestamp":5}],"file_updates":[]}"#.into()),
            "/v1/games/cyberpunk2077/mods/7/files/70/download_link.json" => (200, vec![], r#"[{"URI":"https://cf-files.nexusmods.com/x.zip","name":"CDN","short_name":"CDN"}]"#.into()),
            "/v1/games/cyberpunk2077/mods/8/files/80/download_link.json" => (403, vec![], "{}".into()),
            _ => (404, vec![], "{}".into()),
        });
        let list = files(&api, KEY, 7).unwrap();
        assert_eq!((list[0].file_id, list[0].file_name.as_str()), (70, "Neon-7-1-2-1700000000.zip"));
        assert_eq!(download_link(&api, KEY, 7, 70).unwrap(), "https://cf-files.nexusmods.com/x.zip");
        assert!(matches!(download_link(&api, KEY, 8, 80), Err(AppError::NexusPremium)));
    }

    #[test]
    fn only_https_downloads_from_nexus_servers_are_trusted() {
        assert!(allowed_download_host("https://cf-files.nexusmods.com/cdn/a.zip?md5=x&expires=1"));
        assert!(allowed_download_host("https://supporter-files.nexus-cdn.com/a.zip"));
        assert!(allowed_download_host("https://nexusmods.com/a.zip"));
        for bad in [
            "http://cf-files.nexusmods.com/a.zip",
            "https://evil.example/a.zip",
            "https://nexusmods.com.evil.example/a.zip",
            "https://evilnexusmods.com/a.zip",
            "https://nexusmods.com@evil.example/a.zip",
            "https://cf-files.nexusmods.com/a b.zip",
            "ftp://cf-files.nexusmods.com/a.zip",
            "",
        ] {
            assert!(!allowed_download_host(bad), "{bad}");
        }
    }

    fn local_ok() -> std::sync::Arc<dyn Fn(&str) -> bool + Send + Sync> {
        std::sync::Arc::new(|u: &str| u.starts_with("http://127.0.0.1"))
    }

    fn serve_bytes(body: Vec<u8>) -> u16 {
        let api = serve_raw(body);
        api
    }

    fn serve_raw(body: Vec<u8>) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        port
    }

    #[test]
    fn a_trusted_download_is_saved_and_reports_progress() {
        let port = serve_bytes(b"zip-bytes-here".to_vec());
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        let mut last = (0, 0);
        download_to(&format!("http://127.0.0.1:{port}/a.zip"), &dest, 14, local_ok(), &mut |d, t| last = (d, t)).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"zip-bytes-here");
        assert_eq!(last, (14, 14));
        assert!(!dest.with_extension("part").exists());
    }

    #[test]
    fn an_untrusted_or_oversized_download_is_refused_and_leaves_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        assert!(matches!(
            download_to("https://evil.example/a.zip", &dest, 10, std::sync::Arc::new(allowed_download_host), &mut |_, _| {}),
            Err(AppError::UntrustedHost(_))
        ));
        let port = serve_bytes(vec![1u8; 3 * 1024 * 1024]);
        let err = download_to(&format!("http://127.0.0.1:{port}/a.zip"), &dest, 10, local_ok(), &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, AppError::Network(_)));
        assert!(!dest.exists() && !dest.with_extension("part").exists());
    }

    #[test]
    fn a_redirect_to_another_host_is_not_followed() {
        let api = serve(|_, _| (302, vec![("Location".into(), "http://example.invalid/evil.zip".into())], String::new()));
        let port = api_port(&api);
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("a.zip");
        let err = download_to(&format!("http://127.0.0.1:{port}/a.zip"), &dest, 10, local_ok(), &mut |_, _| {}).unwrap_err();
        assert!(matches!(err, AppError::UntrustedHost(_)));
        assert!(!dest.exists());
    }

    fn api_port(api: &Api) -> u16 {
        api.base.trim_end_matches('/').rsplit(':').next().unwrap().parse().unwrap()
    }

    #[test]
    fn freshness_follows_the_clock() {
        assert!(is_fresh(100, 100, 60));
        assert!(is_fresh(100, 159, 60));
        assert!(!is_fresh(100, 160, 60));
        assert!(!is_fresh(100, 50, 60));
    }

    #[test]
    fn browsing_uses_the_cache_and_serves_old_data_when_nexus_is_busy() {
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let api = serve(move |path, _| {
            counter.fetch_add(1, AtomicOrdering::SeqCst);
            match path {
                "/v1/games/cyberpunk2077.json" => (200, vec![], r#"{"categories":[{"category_id":1,"name":"Visuals","parent_category":false}]}"#.into()),
                "/v1/games/cyberpunk2077/mods/trending.json" => (200, vec![], r#"[{"mod_id":1,"name":"A","version":"1"}]"#.into()),
                _ => (429, vec![], "{}".into()),
            }
        });
        let mut cache = BrowseCache::default();
        let first = browse(&api, KEY, "trending", &mut cache, 1000).unwrap();
        assert_eq!((first.items.len(), first.categories.len(), first.fetched_at), (1, 1, 1000));
        assert_eq!(hits.load(AtomicOrdering::SeqCst), 2);
        browse(&api, KEY, "trending", &mut cache, 1000 + LIST_TTL_SECS - 1).unwrap();
        assert_eq!(hits.load(AtomicOrdering::SeqCst), 2);
        let stale = browse(&api, KEY, "latest_added", &mut cache, 5000);
        assert!(matches!(stale, Err(AppError::RateLimited)));
        cache.lists.insert("latest_added".into(), (10, vec![ModInfo { mod_id: 9, ..ModInfo::default() }]));
        let served = browse(&api, KEY, "latest_added", &mut cache, 5000).unwrap();
        assert_eq!(served.items[0].mod_id, 9);
        assert_eq!(served.fetched_at, 10);
    }

    #[test]
    fn a_rejected_key_stops_browsing() {
        let api = serve(|_, _| (401, vec![], "{}".into()));
        let mut cache = BrowseCache::default();
        assert!(matches!(browse(&api, KEY, "trending", &mut cache, 1), Err(AppError::NexusKey)));
    }

    #[test]
    fn the_schema_probe_saves_the_answer_and_never_the_key() {
        let seen: Arc<Mutex<Vec<(String, Headers)>>> = Arc::default();
        let log = seen.clone();
        let api = serve(move |path, headers| {
            log.lock().unwrap().push((path.to_string(), headers.clone()));
            (200, vec![], r#"{"data":{"__schema":{"queryType":{"name":"Query"}}}}"#.into())
        });
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("schema.json");
        let size = probe_schema(&api, KEY, &dest).unwrap();
        let saved = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(size as usize, saved.len());
        assert!(saved.contains("__schema") && !saved.contains(KEY));
        let seen = seen.lock().unwrap();
        assert_eq!(seen[0].0, "/v2/graphql");
        assert_eq!(seen[0].1.get("apikey").map(String::as_str), Some(KEY));
        assert_eq!(seen[0].1.get("content-type").map(String::as_str), Some("application/json"));
        assert!(matches!(probe_schema(&api, "short", &dest), Err(AppError::NexusKey)));
    }

    fn params() -> SearchParams {
        SearchParams { sort: "downloads".into(), count: 24, ..Default::default() }
    }

    #[test]
    fn search_asks_for_published_cyberpunk_mods_sorted_and_paged() {
        let body = search_body(&SearchParams { offset: 48, ..params() });
        let v = &body["variables"];
        assert_eq!(v["filter"]["gameDomainName"][0]["value"], GAME);
        assert_eq!(v["filter"]["status"][0]["value"], "published");
        assert_eq!(v["sort"][0]["downloads"]["direction"], "DESC");
        assert_eq!((v["offset"].as_u64(), v["count"].as_u64()), (Some(48), Some(24)));
        assert!(v["filter"].get("adultContent").is_none() && v["filter"].get("nameStemmed").is_none());
    }

    #[test]
    fn search_filters_are_added_only_when_chosen() {
        let body = search_body(&SearchParams {
            query: "  neon * \"x\"?  ".into(),
            hide_adult: true,
            tag: Some("Cars".into()),
            categories: vec!["Visuals".into()],
            ..params()
        });
        let f = &body["variables"]["filter"];
        assert_eq!(f["nameStemmed"][0]["value"], "neon x");
        assert_eq!(f["nameStemmed"][0]["op"], "MATCHES");
        assert_eq!(f["adultContent"][0]["value"], false);
        assert_eq!(f["tag"][0]["value"], "Cars");
        assert_eq!(f["categoryName"][0]["value"], "Visuals");
        let many = search_body(&SearchParams { categories: vec!["A".into(), "B".into()], ..params() });
        let any = &many["variables"]["filter"]["filter"][0];
        assert_eq!(any["op"], "OR");
        assert_eq!(any["filter"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn search_rejects_odd_requests_before_they_leave_the_computer() {
        assert!(check_search(&params()).is_ok());
        for bad in [
            SearchParams { sort: "random".into(), ..params() },
            SearchParams { count: 0, ..params() },
            SearchParams { count: 500, ..params() },
            SearchParams { offset: 99_999, ..params() },
            SearchParams { tag: Some(String::new()), ..params() },
            SearchParams { categories: vec![String::new()], ..params() },
        ] {
            assert!(check_search(&bad).is_err());
        }
    }

    #[test]
    fn search_answers_become_cards_and_odd_nodes_are_skipped() {
        let v = serde_json::json!({"data":{"mods":{"totalCount":1234,"nodes":[
            {"modId":7,"name":"Neon","summary":"s","version":"1.2","author":"a","pictureUrl":null,"thumbnailUrl":"https://x/t.jpg",
             "downloads":10,"endorsements":3,"adultContent":true,"status":"published","modCategory":{"categoryId":5},"tags":[{"name":"Cars"},{"name":"UI"}]},
            {"name":"no id"}
        ]}}});
        let page = parse_search(&v).unwrap();
        assert_eq!(page.total, 1234);
        assert_eq!(page.items.len(), 1);
        let m = &page.items[0];
        assert_eq!((m.mod_id, m.category_id, m.mod_downloads, m.endorsement_count), (7, 5, 10, 3));
        assert_eq!(m.picture_url.as_deref(), Some("https://x/t.jpg"));
        assert!(m.contains_adult_content);
        assert_eq!(m.tags, vec!["Cars", "UI"]);
    }

    #[test]
    fn search_errors_from_nexus_are_reported_briefly() {
        let v = serde_json::json!({"errors":[{"message":"Unknown field"}]});
        assert!(matches!(parse_search(&v), Err(AppError::Network(m)) if m.contains("Unknown field")));
        assert!(parse_search(&serde_json::json!({})).is_err());
    }

    #[test]
    fn the_memory_store_behaves_like_the_real_one() {
        let store = MemStore(Mutex::new(None));
        assert_eq!(store.get().unwrap(), None);
        store.set(KEY).unwrap();
        assert_eq!(store.get().unwrap().as_deref(), Some(KEY));
        store.delete().unwrap();
        assert_eq!(store.get().unwrap(), None);
        store.delete().unwrap();
    }
}
