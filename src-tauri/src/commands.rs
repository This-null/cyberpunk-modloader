use crate::archive;
use crate::error::AppError;
use crate::game;
use crate::library::{Library, ModRecord};
use crate::report::{self, ReportInput};
use crate::logs::{self, Chunk, LogFile};
use crate::nexus;
use crate::nightcoop::{self, Endpoint, ReleaseView};
use crate::nxm;
use crate::profile;
use crate::scan::{self, ExternalMod};
use crate::version::{self, BuildInfo, Detected};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub data: PathBuf,
}

#[derive(Serialize, Deserialize, Default)]
struct Settings {
    game_dir: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    motion: Option<String>,
    #[serde(default)]
    translate: Option<String>,
    #[serde(default)]
    updates: Option<String>,
    #[serde(default)]
    translate_lang: Option<String>,
    #[serde(default)]
    install_id: Option<String>,
}

#[derive(Serialize)]
pub struct Prefs {
    locale: Option<String>,
    theme: Option<String>,
    motion: Option<String>,
    translate: Option<String>,
    updates: Option<String>,
    translate_lang: Option<String>,
}

pub fn write_atomic(path: PathBuf, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, &path)
}

fn same_dir(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_ascii_lowercase();
    norm(a) == norm(b)
}

fn settings_path(data: &Path) -> PathBuf {
    data.join("settings.json")
}

fn load_settings(data: &Path) -> Settings {
    fs::read_to_string(settings_path(data))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_settings(data: &Path, s: &Settings) -> Result<(), AppError> {
    fs::create_dir_all(data)?;
    let text = serde_json::to_string_pretty(s).map_err(|e| AppError::State(e.to_string()))?;
    write_atomic(settings_path(data), text)?;
    Ok(())
}

fn library(state: &AppState) -> Result<Library, AppError> {
    let game = load_settings(&state.data)
        .game_dir
        .ok_or(AppError::GameNotFound)?;
    Ok(Library::new(state.data.join("mod_library"), PathBuf::from(game)))
}

pub fn guard_not_running(running: bool) -> Result<(), AppError> {
    if running {
        Err(AppError::GameRunning)
    } else {
        Ok(())
    }
}

fn logged<T>(r: Result<T, AppError>) -> Result<T, AppError> {
    if let Err(e) = &r {
        tracing::error!("{e}");
    }
    r
}

pub fn tail_lines(text: &str, n: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].iter().map(|l| l.to_string()).collect()
}

fn newest_log_tail(data: &Path, n: usize) -> Vec<String> {
    let Ok(entries) = fs::read_dir(data.join("logs")) else {
        return Vec::new();
    };
    let newest = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t);
    match newest.and_then(|(_, p)| fs::read_to_string(p).ok()) {
        Some(text) => tail_lines(&text, n),
        None => Vec::new(),
    }
}

#[tauri::command]
pub fn detect_games() -> Vec<String> {
    game::find_game_dirs()
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[tauri::command]
pub fn get_game_dir(state: State<'_, AppState>) -> Option<String> {
    load_settings(&state.data).game_dir
}

#[tauri::command]
pub fn get_prefs(state: State<'_, AppState>) -> Prefs {
    let s = load_settings(&state.data);
    Prefs { locale: s.locale, theme: s.theme, motion: s.motion, translate: s.translate, updates: s.updates, translate_lang: s.translate_lang }
}

#[tauri::command]
pub fn set_prefs(
    state: State<'_, AppState>,
    locale: String,
    theme: String,
    motion: String,
    translate: String,
    updates: String,
    translate_lang: String,
) -> Result<(), AppError> {
    const LOCALES: [&str; 9] = ["auto", "en", "tr", "ru", "es", "pt-BR", "fr", "pl", "de"];
    if !LOCALES.contains(&locale.as_str())
        || !["system", "dark", "light"].contains(&theme.as_str())
        || !["system", "full", "reduced"].contains(&motion.as_str())
        || !["ask", "on", "off"].contains(&translate.as_str())
        || !["on", "off"].contains(&updates.as_str())
        || !(translate_lang == "app" || LOCALES[1..].contains(&translate_lang.as_str()))
    {
        return Err(AppError::State("unknown preference".into()));
    }
    let mut s = load_settings(&state.data);
    s.locale = Some(locale);
    s.theme = Some(theme);
    s.motion = Some(motion);
    s.translate = Some(translate);
    s.updates = Some(updates);
    s.translate_lang = Some(translate_lang);
    save_settings(&state.data, &s)
}

#[tauri::command]
pub fn set_game_dir(state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    if !game::is_valid_game_dir(Path::new(&path)) {
        return logged(Err(AppError::GameNotFound));
    }
    let mut s = load_settings(&state.data);
    if let Some(old) = &s.game_dir {
        if !same_dir(old, &path) {
            if let Ok(lib) = library(&state) {
                if lib.list()?.iter().any(|m| m.enabled) {
                    return logged(Err(AppError::ModsEnabled));
                }
            }
        }
    }
    s.game_dir = Some(path);
    logged(save_settings(&state.data, &s))
}

#[tauri::command]
pub fn game_running() -> bool {
    game::is_game_running()
}

#[tauri::command]
pub fn list_mods(state: State<'_, AppState>) -> Result<Vec<ModRecord>, AppError> {
    logged(library(&state)?.list())
}

#[tauri::command]
pub fn scan_external(state: State<'_, AppState>) -> Result<Vec<ExternalMod>, AppError> {
    let lib = library(&state)?;
    let records = lib.list()?;
    Ok(scan::scan_external(lib.game_dir(), &records))
}

#[tauri::command]
pub fn import_mod(state: State<'_, AppState>, name: String) -> Result<ModRecord, AppError> {
    let lib = library(&state)?;
    let records = lib.list()?;
    let found = scan::scan_external(lib.game_dir(), &records)
        .into_iter()
        .find(|m| m.name == name)
        .ok_or_else(|| AppError::NotFound(name.clone()))?;
    logged(lib.adopt(&found.name, &found.files))
}

#[derive(Serialize)]
pub struct NcData {
    updated: String,
    latest: String,
    releases: Vec<ReleaseView>,
    offline: bool,
}

fn nc_cache(data: &Path) -> PathBuf {
    data.join("nightcoop-releases.json")
}

fn nc_data(view: nightcoop::ReleasesView, offline: bool) -> NcData {
    NcData {
        updated: view.updated,
        latest: view.latest,
        releases: view.releases,
        offline,
    }
}

#[tauri::command]
pub async fn nc_fetch(state: State<'_, AppState>, lang: String) -> Result<NcData, AppError> {
    let data = state.data.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let endpoint = Endpoint::official();
        let fresh = nightcoop::fetch_text(&endpoint, nightcoop::RELEASES_PATH)
            .and_then(|text| nightcoop::parse_releases(&text).map(|r| (text, r)));
        match fresh {
            Ok((text, releases)) => {
                let _ = write_atomic(nc_cache(&data), text);
                Ok(nc_data(nightcoop::to_view(&releases, &lang), false))
            }
            Err(e) => {
                tracing::warn!("{e}");
                let text = fs::read_to_string(nc_cache(&data)).map_err(|_| e)?;
                let releases = nightcoop::parse_releases(&text)?;
                Ok(nc_data(nightcoop::to_view(&releases, &lang), true))
            }
        }
    })
    .await
    .map_err(|e| AppError::Io(e.to_string()))?
}

#[tauri::command]
pub async fn nc_install(
    app: AppHandle,
    state: State<'_, AppState>,
    version: String,
    lang: String,
    force: bool,
) -> Result<(), AppError> {
    guard_not_running(game::is_game_running())?;
    let data = state.data.clone();
    let lib = library(&state)?;
    let outcome = tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let endpoint = Endpoint::official();
        let text = nightcoop::fetch_text(&endpoint, nightcoop::RELEASES_PATH)?;
        let releases = nightcoop::parse_releases(&text)?;
        let release = releases
            .releases
            .iter()
            .find(|r| r.version == version)
            .ok_or_else(|| AppError::ReleaseNotFound(version.clone()))?;
        let view = nightcoop::to_view(&releases, &lang);
        let shown = view
            .releases
            .iter()
            .find(|r| r.version == version)
            .ok_or_else(|| AppError::ReleaseNotFound(version.clone()))?;
        if let Some(reason) = &shown.yanked {
            return Err(AppError::Withdrawn(reason.clone()));
        }
        let file = nightcoop::mod_file(release).ok_or_else(|| AppError::ReleaseNotFound(version.clone()))?;
        let safe_name = !file.name.is_empty()
            && file
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c))
            && !file.name.contains("..");
        if !safe_name {
            return Err(AppError::UnsafePath(file.name.clone()));
        }
        let name = format!("NightCoop {version}");
        if !lib.list()?.iter().any(|r| r.name == name) {
            let dest = data.join("downloads").join(&file.name);
            let mut last = u64::MAX;
            let mut progress = |done: u64, total: u64| {
                let pct = if total == 0 { 0 } else { done * 100 / total };
                if pct != last {
                    last = pct;
                    let _ = app.emit("nc-progress", serde_json::json!({ "done": done, "total": total }));
                }
            };
            nightcoop::download(&endpoint, &file.url, &dest, file.size, &file.sha256, &mut progress)?;
            let installed = lib.install_any(&dest, &name);
            let _ = fs::remove_file(&dest);
            installed?;
            lib.tag_record(&name, "nightcoop", &version, &shown.title)?;
        }
        lib.activate_exclusive(&name, force)
    })
    .await
    .map_err(|e| AppError::Io(e.to_string()))?;
    match outcome {
        Err(AppError::Conflict(list)) => Err(AppError::Conflict(list)),
        other => logged(other),
    }
}

pub struct NexusState {
    pub store: Box<dyn nexus::KeyStore>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct NexusUser {
    name: String,
    is_premium: bool,
    is_supporter: bool,
}

#[derive(Serialize)]
pub struct NexusStatus {
    connected: bool,
    user: Option<NexusUser>,
}

fn nexus_user_path(data: &Path) -> PathBuf {
    data.join("nexus-user.json")
}

fn nexus_cache_path(data: &Path) -> PathBuf {
    data.join("nexus-cache.json")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load_nexus_user(data: &Path) -> Option<NexusUser> {
    fs::read_to_string(nexus_user_path(data))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
}

fn load_nexus_cache(data: &Path) -> nexus::Cache {
    fs::read_to_string(nexus_cache_path(data))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn allowed_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let clean = url.len() <= 300 && !url.chars().any(|c| c.is_control() || c.is_whitespace());
    let repo = url.starts_with("https://github.com/This-null/");
    clean && (["www.nexusmods.com", "nexusmods.com", "nightcoop.net"].contains(&host) || repo)
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), AppError> {
    if !allowed_url(&url) {
        return Err(AppError::BadLink(url.chars().take(80).collect()));
    }
    let _ = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", &url])
        .spawn();
    Ok(())
}

#[tauri::command]
pub fn nexus_status(state: State<'_, AppState>, nx: State<'_, NexusState>) -> Result<NexusStatus, AppError> {
    let connected = nx.store.get()?.is_some();
    Ok(NexusStatus {
        connected,
        user: if connected { load_nexus_user(&state.data) } else { None },
    })
}

#[tauri::command]
pub async fn nexus_connect(
    state: State<'_, AppState>,
    nx: State<'_, NexusState>,
    key: String,
) -> Result<NexusStatus, AppError> {
    let key = key.trim().to_string();
    if !nexus::valid_key(&key) {
        return logged(Err(AppError::NexusKey));
    }
    let probe = key.clone();
    let user = tauri::async_runtime::spawn_blocking(move || nexus::validate(&nexus::Api::official(), &probe))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    let user = logged(user)?;
    nx.store.set(&key)?;
    let user = NexusUser {
        name: user.name,
        is_premium: user.is_premium,
        is_supporter: user.is_supporter,
    };
    if let Ok(text) = serde_json::to_string(&user) {
        let _ = write_atomic(nexus_user_path(&state.data), text);
    }
    Ok(NexusStatus {
        connected: true,
        user: Some(user),
    })
}

#[tauri::command]
pub fn nexus_disconnect(state: State<'_, AppState>, nx: State<'_, NexusState>) -> Result<(), AppError> {
    nx.store.delete()?;
    let _ = fs::remove_file(nexus_user_path(&state.data));
    let _ = fs::remove_file(nexus_cache_path(&state.data));
    Ok(())
}

#[tauri::command]
pub fn nexus_link(state: State<'_, AppState>, name: String, target: String) -> Result<ModRecord, AppError> {
    let id = nexus::parse_target(&target)
        .ok_or_else(|| AppError::BadLink(target.chars().take(80).collect()))?;
    logged(library(&state)?.set_nexus_id(&name, Some(id)))
}

#[tauri::command]
pub fn nexus_unlink(state: State<'_, AppState>, name: String) -> Result<ModRecord, AppError> {
    logged(library(&state)?.set_nexus_id(&name, None))
}

#[tauri::command]
pub async fn nexus_check(
    state: State<'_, AppState>,
    nx: State<'_, NexusState>,
) -> Result<Vec<nexus::Status>, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    let records = library(&state)?.list()?;
    let data = state.data.clone();
    let done = tauri::async_runtime::spawn_blocking(move || {
        let mut cache = load_nexus_cache(&data);
        let statuses = nexus::check_all(
            &nexus::Api::official(),
            &key,
            &records,
            &mut cache,
            now_secs(),
            nexus::CHECK_TTL_SECS,
        );
        if let Ok(text) = serde_json::to_string(&cache) {
            let _ = write_atomic(nexus_cache_path(&data), text);
        }
        statuses
    })
    .await
    .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[derive(Serialize)]
pub struct InstallResult {
    added: Vec<ModRecord>,
    skipped: Vec<String>,
}

fn install_archive(lib: &Library, data: &Path, path: &Path, name: &str) -> Result<Vec<ModRecord>, AppError> {
    install_archive_report(lib, data, path, name).map(|r| r.0)
}

fn install_archive_report(
    lib: &Library,
    data: &Path,
    path: &Path,
    name: &str,
) -> Result<(Vec<ModRecord>, Vec<String>), AppError> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let work = data.join("tmp").join(stamp.to_string());
    let done = archive::to_zip(path, &work, crate::library::Limits::default()).and_then(|zip| lib.install_report(&zip, name));
    let _ = fs::remove_dir_all(&work);
    done
}

fn safe_zip_name(name: &str) -> Result<String, AppError> {
    let ok = !name.is_empty()
        && name.len() <= 120
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " ._-+()".contains(c));
    if !ok {
        return Err(AppError::UnsafePath(name.chars().take(80).collect()));
    }
    if archive::kind(Path::new(name)).is_none() {
        return Err(AppError::Zip("only .zip, .rar and .7z files can be installed".into()));
    }
    Ok(name.to_string())
}

fn mod_folder_name(name: Option<&str>, mod_id: u64) -> String {
    let cleaned: String = name
        .unwrap_or("")
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let cleaned: String = cleaned.trim().trim_start_matches('.').chars().take(56).collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        format!("Nexus mod {mod_id}")
    } else {
        cleaned
    }
}

#[derive(Serialize)]
pub struct NexusBrowse {
    items: Vec<nexus::ModInfo>,
    categories: Vec<nexus::Category>,
    fetched_at: u64,
    installed: Vec<u64>,
}

fn browse_cache_path(data: &Path) -> PathBuf {
    data.join("nexus-browse.json")
}

#[tauri::command]
pub async fn nexus_browse(
    state: State<'_, AppState>,
    nx: State<'_, NexusState>,
    kind: String,
) -> Result<NexusBrowse, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    let installed: Vec<u64> = library(&state)
        .and_then(|l| l.list())
        .map(|records| records.into_iter().filter_map(|m| m.nexus_id).collect())
        .unwrap_or_default();
    let data = state.data.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut cache: nexus::BrowseCache = fs::read_to_string(browse_cache_path(&data))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let out = nexus::browse(&nexus::Api::official(), &key, &kind, &mut cache, now_secs());
        if let Ok(text) = serde_json::to_string(&cache) {
            let _ = write_atomic(browse_cache_path(&data), text);
        }
        out
    })
    .await
    .map_err(|e| AppError::Io(e.to_string()))?;
    let data = logged(result)?;
    Ok(NexusBrowse {
        items: data.items,
        categories: data.categories,
        fetched_at: data.fetched_at,
        installed,
    })
}

#[tauri::command]
pub async fn translate_notes(state: State<'_, AppState>, lines: Vec<String>, lang: String) -> Result<Vec<String>, AppError> {
    let cache = state.data.join("translations.json");
    let done = tauri::async_runtime::spawn_blocking(move || crate::translate::translate_lines(&cache, &lines, &lang))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[tauri::command]
pub async fn nexus_requirements(nx: State<'_, NexusState>, mod_id: u64) -> Result<Vec<nexus::Requirement>, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    let done = tauri::async_runtime::spawn_blocking(move || nexus::requirements(&nexus::Api::official(), &key, mod_id))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[derive(Serialize)]
pub struct ProfileBuilt {
    code: String,
    json: String,
    count: usize,
    skipped: Vec<String>,
}

#[tauri::command]
pub fn profile_build(state: State<'_, AppState>, names: Vec<String>, title: String) -> Result<ProfileBuilt, AppError> {
    let records = library(&state)?.list()?;
    let built = profile::build(&records, &names, &title);
    if built.profile.mods.is_empty() {
        return logged(Err(AppError::Unmappable("nothing to share".into())));
    }
    Ok(ProfileBuilt {
        code: profile::encode(&built.profile)?,
        json: serde_json::to_string_pretty(&built.profile).map_err(|e| AppError::State(e.to_string()))?,
        count: built.profile.mods.len(),
        skipped: built.skipped,
    })
}

#[derive(Serialize)]
pub struct ProfileView {
    name: String,
    items: Vec<profile::PlanItem>,
    encoded: String,
}

#[tauri::command]
pub async fn profile_open(state: State<'_, AppState>, text: String) -> Result<ProfileView, AppError> {
    let lib = library(&state)?;
    let done = tauri::async_runtime::spawn_blocking(move || -> Result<ProfileView, AppError> {
        let raw = if profile::is_short_code(&text) { profile::fetch(profile::SERVER, &text)? } else { text };
        let p = profile::decode(&raw)?;
        let items = profile::plan(&p, &lib.list()?);
        Ok(ProfileView { name: p.name.clone(), items, encoded: profile::encode(&p)? })
    })
    .await
    .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

fn install_id(data: &Path) -> Result<String, AppError> {
    let mut s = load_settings(data);
    if let Some(id) = s.install_id.clone().filter(|i| i.len() == 36) {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    s.install_id = Some(id.clone());
    save_settings(data, &s)?;
    Ok(id)
}

#[tauri::command]
pub async fn profile_share(state: State<'_, AppState>, encoded: String) -> Result<profile::Shared, AppError> {
    let id = install_id(&state.data)?;
    let done = tauri::async_runtime::spawn_blocking(move || profile::share(profile::SERVER, &encoded, &id))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[tauri::command]
pub fn profile_save(path: String, text: String) -> Result<(), AppError> {
    let p = PathBuf::from(&path);
    let ok = text.len() <= 256 * 1024 && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json") || e.eq_ignore_ascii_case("cp2077profile"));
    if !ok {
        return logged(Err(AppError::UnsafePath(path.chars().take(80).collect())));
    }
    logged(write_atomic(p, text).map_err(AppError::from))
}

#[tauri::command]
pub fn profile_read(path: String) -> Result<String, AppError> {
    let p = PathBuf::from(&path);
    let ok = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json") || e.eq_ignore_ascii_case("cp2077profile"))
        && fs::metadata(&p).is_ok_and(|m| m.is_file() && m.len() <= 256 * 1024);
    if !ok {
        return logged(Err(AppError::UnsafePath(path.chars().take(80).collect())));
    }
    logged(fs::read_to_string(&p).map_err(AppError::from))
}

#[tauri::command]
pub async fn nexus_search(nx: State<'_, NexusState>, params: nexus::SearchParams) -> Result<nexus::SearchPage, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    let done = tauri::async_runtime::spawn_blocking(move || nexus::search(&nexus::Api::official(), &key, &params))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

type ForcedFile = Option<(u64, Option<(String, u64)>)>;

fn run_nexus_install(
    app: &AppHandle,
    lib: &Library,
    data: &Path,
    key: &str,
    mod_id: u64,
    forced: ForcedFile,
) -> Result<Vec<String>, AppError> {
    let api = nexus::Api::official();
    let info = nexus::mod_info(&api, key, mod_id)?;
    let files = nexus::files(&api, key, mod_id)?;
    let (file, link_auth) = match forced {
        Some((file_id, auth)) => (
            files
                .iter()
                .find(|f| f.file_id == file_id)
                .ok_or_else(|| AppError::NotFound("file".into()))?,
            auth,
        ),
        None => (nexus::choose_file(&files).ok_or_else(|| AppError::NotFound("file".into()))?, None),
    };
    let zip_name = safe_zip_name(&file.file_name)?;
    let link = nexus::download_link_with(
        &api,
        key,
        mod_id,
        file.file_id,
        link_auth.as_ref().map(|(k, e)| (k.as_str(), *e)),
    )?;
    let dest = data.join("downloads").join(&zip_name);
    let mut last = u64::MAX;
    let mut progress = |done: u64, total: u64| {
        let pct = if total == 0 { 0 } else { done * 100 / total };
        if pct != last {
            last = pct;
            let _ = app.emit("nexus-progress", serde_json::json!({ "done": done, "total": total }));
        }
    };
    nexus::download_to(
        &link,
        &dest,
        file.size_kb * 1024,
        std::sync::Arc::new(nexus::allowed_download_host),
        &mut progress,
    )?;
    let installed = install_archive(lib, data, &dest, &mod_folder_name(info.name.as_deref(), mod_id));
    let _ = fs::remove_file(&dest);
    let records = installed?;
    for rec in &records {
        lib.set_nexus_id(&rec.name, Some(mod_id))?;
        let description = if rec.description.is_empty() {
            info.summary.clone().unwrap_or_default()
        } else {
            rec.description.clone()
        };
        let version = if rec.version.is_empty() { file.version.clone() } else { rec.version.clone() };
        lib.set_info(&rec.name, &description, &version, None)?;
    }
    let names: Vec<String> = records.into_iter().map(|r| r.name).collect();
    lib.replace_older(&names);
    Ok(names)
}

#[tauri::command]
pub async fn nexus_install(
    app: AppHandle,
    state: State<'_, AppState>,
    nx: State<'_, NexusState>,
    mod_id: u64,
) -> Result<Vec<String>, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    if !load_nexus_user(&state.data).is_some_and(|u| u.is_premium) {
        return logged(Err(AppError::NexusPremium));
    }
    let lib = library(&state)?;
    let data = state.data.clone();
    let done = tauri::async_runtime::spawn_blocking(move || run_nexus_install(&app, &lib, &data, &key, mod_id, None))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[tauri::command]
pub async fn nexus_install_nxm(
    app: AppHandle,
    state: State<'_, AppState>,
    nx: State<'_, NexusState>,
    link: nxm::NxmLink,
) -> Result<Vec<String>, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    let checked = nxm::parse(&format!(
        "nxm://{}/mods/{}/files/{}?key={}&expires={}",
        nexus::GAME,
        link.mod_id,
        link.file_id,
        link.key,
        link.expires
    ))?;
    let lib = library(&state)?;
    let data = state.data.clone();
    let done = tauri::async_runtime::spawn_blocking(move || {
        run_nexus_install(
            &app,
            &lib,
            &data,
            &key,
            checked.mod_id,
            Some((checked.file_id, Some((checked.key, checked.expires)))),
        )
    })
    .await
    .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[tauri::command]
pub async fn nexus_mod_name(nx: State<'_, NexusState>, mod_id: u64) -> Result<String, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    let done = tauri::async_runtime::spawn_blocking(move || nexus::mod_info(&nexus::Api::official(), &key, mod_id))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    Ok(logged(done)?.name.unwrap_or_default())
}

pub struct NxmState {
    pub pending: std::sync::Mutex<Option<nxm::NxmLink>>,
}

#[tauri::command]
pub fn take_pending_nxm(state: State<'_, NxmState>) -> Option<nxm::NxmLink> {
    state.pending.lock().unwrap_or_else(|e| e.into_inner()).take()
}

#[tauri::command]
pub fn nxm_status() -> Result<nxm::Handler, AppError> {
    let exe = std::env::current_exe().map_err(|e| AppError::Io(e.to_string()))?;
    Ok(nxm::status(&exe))
}

#[tauri::command]
pub fn nxm_register(enable: bool) -> Result<nxm::Handler, AppError> {
    let exe = std::env::current_exe().map_err(|e| AppError::Io(e.to_string()))?;
    logged(if enable { nxm::register(&exe) } else { nxm::unregister(&exe) })?;
    Ok(nxm::status(&exe))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[tauri::command]
pub fn downloads_recent(since: u64) -> Vec<nxm::Found> {
    dirs::download_dir()
        .map(|d| nxm::recent_archives(&d, since, now_unix()))
        .unwrap_or_default()
}

#[tauri::command]
pub async fn install_download(state: State<'_, AppState>, path: String) -> Result<Vec<ModRecord>, AppError> {
    let file = PathBuf::from(&path);
    let allowed = dirs::download_dir().is_some_and(|d| nxm::is_inside(&d, &file)) && archive::kind(&file).is_some();
    if !allowed {
        return logged(Err(AppError::UnsafePath(path.chars().take(80).collect())));
    }
    let lib = library(&state)?;
    let data = state.data.clone();
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
    let done = tauri::async_runtime::spawn_blocking(move || {
        let added = install_archive(&lib, &data, &file, &mod_folder_name(stem.as_deref(), 0))?;
        lib.replace_older(&added.iter().map(|r| r.name.clone()).collect::<Vec<_>>());
        Ok(added)
    })
    .await
    .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[tauri::command]
pub async fn nexus_probe(state: State<'_, AppState>, nx: State<'_, NexusState>) -> Result<u64, AppError> {
    let key = nx.store.get()?.ok_or(AppError::NexusKey)?;
    let dest = state.data.join("nexus-schema.json");
    let done = tauri::async_runtime::spawn_blocking(move || nexus::probe_schema(&nexus::Api::official(), &key, &dest))
        .await
        .map_err(|e| AppError::Io(e.to_string()))?;
    logged(done)
}

#[tauri::command]
pub fn install_mod(
    state: State<'_, AppState>,
    zip_path: String,
    name: String,
) -> Result<InstallResult, AppError> {
    let lib = library(&state)?;
    let (added, skipped) = logged(install_archive_report(&lib, &state.data, Path::new(&zip_path), &name))?;
    lib.replace_older(&added.iter().map(|r| r.name.clone()).collect::<Vec<_>>());
    Ok(InstallResult { added, skipped })
}

#[tauri::command]
pub fn set_mod_info(
    state: State<'_, AppState>,
    name: String,
    description: String,
    version: String,
    image_path: Option<String>,
) -> Result<ModRecord, AppError> {
    logged(library(&state)?.set_info(
        &name,
        &description,
        &version,
        image_path.as_deref().map(Path::new),
    ))
}

#[tauri::command]
pub fn mod_image(state: State<'_, AppState>, name: String) -> Result<Option<String>, AppError> {
    library(&state)?.image_data_url(&name)
}

#[tauri::command]
pub fn export_mod(state: State<'_, AppState>, name: String, dest: String) -> Result<(), AppError> {
    logged(library(&state)?.export_mod(&name, Path::new(&dest)))
}

#[tauri::command]
pub fn export_pack(
    state: State<'_, AppState>,
    names: Vec<String>,
    dest: String,
) -> Result<(), AppError> {
    logged(library(&state)?.export_pack(&names, Path::new(&dest)))
}

#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    data_dir: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.data.to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub fn enable_mod(state: State<'_, AppState>, name: String, force: bool) -> Result<(), AppError> {
    guard_not_running(game::is_game_running())?;
    logged(library(&state)?.activate_exclusive(&name, force))
}

#[tauri::command]
pub fn disable_mod(state: State<'_, AppState>, name: String) -> Result<(), AppError> {
    guard_not_running(game::is_game_running())?;
    logged(library(&state)?.disable(&name))
}

#[tauri::command]
pub fn uninstall_mod(state: State<'_, AppState>, name: String) -> Result<(), AppError> {
    guard_not_running(game::is_game_running())?;
    logged(library(&state)?.uninstall(&name))
}

fn game_opt(state: &AppState) -> Option<PathBuf> {
    load_settings(&state.data).game_dir.map(PathBuf::from)
}

#[derive(Serialize)]
pub struct CrashSummary {
    id: String,
    name: String,
    source: String,
    modified: u64,
    excerpt: String,
}

#[tauri::command]
pub fn logs_scan(state: State<'_, AppState>) -> Vec<LogFile> {
    let game = game_opt(&state);
    logs::scan(game.as_deref(), &state.data)
}

#[tauri::command]
pub fn logs_read(
    state: State<'_, AppState>,
    id: String,
    end: Option<u64>,
    max: Option<u64>,
) -> Result<Chunk, AppError> {
    let game = game_opt(&state);
    let path = logs::resolve(game.as_deref(), &state.data, &id)?;
    logs::read_chunk(&path, end, max.unwrap_or(logs::DEFAULT_CHUNK))
}

#[tauri::command]
pub fn logs_head(state: State<'_, AppState>, id: String) -> Result<Chunk, AppError> {
    let game = game_opt(&state);
    let path = logs::resolve(game.as_deref(), &state.data, &id)?;
    logs::read_head(&path, logs::DEFAULT_CHUNK)
}

#[tauri::command]
pub fn logs_poll(state: State<'_, AppState>, id: String, from: u64) -> Result<Chunk, AppError> {
    let game = game_opt(&state);
    let path = logs::resolve(game.as_deref(), &state.data, &id)?;
    logs::read_since(&path, from, logs::DEFAULT_CHUNK)
}

#[tauri::command]
pub fn logs_summary(state: State<'_, AppState>) -> Result<Option<CrashSummary>, AppError> {
    let game = game_opt(&state);
    let Some(file) = logs::scan(game.as_deref(), &state.data).into_iter().find(|f| f.crash) else {
        return Ok(None);
    };
    let path = logs::resolve(game.as_deref(), &state.data, &file.id)?;
    Ok(Some(CrashSummary {
        excerpt: logs::excerpt(&path, 12)?,
        id: file.id,
        name: file.name,
        source: file.source,
        modified: file.modified,
    }))
}

#[tauri::command]
pub fn logs_reveal(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let game = game_opt(&state);
    let path = logs::resolve(game.as_deref(), &state.data, &id)?;
    open_folder(path.to_string_lossy().into_owned());
    Ok(())
}

#[tauri::command]
pub fn logs_clear(state: State<'_, AppState>) -> Result<logs::Cleared, AppError> {
    let game = game_opt(&state);
    Ok(logs::clear(game.as_deref(), &state.data))
}

#[tauri::command]
pub async fn nc_detect(
    state: State<'_, AppState>,
    builds: Vec<BuildInfo>,
) -> Result<Option<Detected>, AppError> {
    let Some(game) = game_opt(&state) else {
        return Ok(None);
    };
    tauri::async_runtime::spawn_blocking(move || version::detect_nightcoop(&game, &builds))
        .await
        .map_err(|e| AppError::Io(e.to_string()))
}

#[tauri::command]
pub fn build_error_report(
    state: State<'_, AppState>,
    error: String,
    note: String,
    language: String,
    log_id: Option<String>,
) -> String {
    let mods = library(&state)
        .and_then(|l| l.list())
        .map(|list| {
            list.into_iter()
                .filter(|m| m.enabled)
                .map(|m| m.name)
                .collect()
        })
        .unwrap_or_default();
    let input = ReportInput {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os: sysinfo::System::long_os_version().unwrap_or_else(|| "unknown".into()),
        language,
        error,
        log_tail: report_log_tail(&state, log_id.as_deref()),
        mods,
        note,
    };
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    report::build_report(&input, &home)
}

fn report_log_tail(state: &AppState, log_id: Option<&str>) -> Vec<String> {
    let mut lines = newest_log_tail(&state.data, 200);
    let Some(id) = log_id else {
        return lines;
    };
    let game = game_opt(state);
    if let Ok(path) = logs::resolve(game.as_deref(), &state.data, id) {
        if let Ok(chunk) = logs::read_chunk(&path, None, 64 * 1024) {
            lines.push(format!("--- {id} (last 200 lines) ---"));
            lines.extend(tail_lines(&chunk.text, 200));
        }
    }
    lines
}

#[tauri::command]
pub fn save_error_report(text: String) -> Result<String, AppError> {
    let dir = dirs::desktop_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| AppError::Io("no folder to save into".into()))?;
    let path = logged(report::save_report(&text, &dir))?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(windows)]
fn explorer(arg: String) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("explorer").raw_arg(arg).spawn();
}

#[cfg(not(windows))]
fn explorer(_: String) {}

fn explorer_path(path: &str) -> Option<String> {
    let fixed = path.replace('/', "\\");
    (!fixed.is_empty() && !fixed.contains(['"', '\n', '\r', '|', '<', '>']) && Path::new(&fixed).exists()).then_some(fixed)
}

#[tauri::command]
pub fn open_folder(path: String) {
    if let Some(p) = explorer_path(&path) {
        explorer(format!("/select,\"{p}\""));
    }
}

#[tauri::command]
pub fn logs_open(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let game = game_opt(&state);
    let path = logs::resolve(game.as_deref(), &state.data, &id)?;
    if let Some(p) = explorer_path(&path.to_string_lossy()) {
        explorer(format!("\"{p}\""));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_zip_names_may_be_written_to_the_downloads_folder() {
        assert_eq!(safe_zip_name("Neon Streets-7-1-2-1700000000.zip").unwrap(), "Neon Streets-7-1-2-1700000000.zip");
        for bad in ["", "../x.zip", "a/b.zip", "a\\b.zip", "C:x.zip", "x.zip.exe", "x..zip", "a;b.zip"] {
            assert!(safe_zip_name(bad).is_err(), "{bad}");
        }
        assert_eq!(safe_zip_name("pack.7z").unwrap(), "pack.7z");
        assert_eq!(safe_zip_name("pack.RAR").unwrap(), "pack.RAR");
        assert!(matches!(safe_zip_name("pack.exe"), Err(AppError::Zip(_))));
    }

    #[test]
    fn mod_names_become_safe_folder_names() {
        assert_eq!(mod_folder_name(Some("Neon: Streets?"), 7), "Neon_ Streets_");
        assert_eq!(mod_folder_name(Some("  ..hidden"), 7), "hidden");
        assert_eq!(mod_folder_name(None, 7), "Nexus mod 7");
        assert_eq!(mod_folder_name(Some("***"), 7), "___");
        assert!(mod_folder_name(Some(&"x".repeat(200)), 7).len() <= 56);
    }

    #[test]
    fn only_https_links_to_known_sites_may_be_opened() {
        assert!(allowed_url("https://www.nexusmods.com/users/myaccount?tab=api"));
        assert!(allowed_url("https://www.nexusmods.com/cyberpunk2077/mods/107"));
        assert!(allowed_url("https://nightcoop.net/download.html"));
        assert!(allowed_url("https://github.com/This-null/cyberpunk-modloader"));
        for bad in [
            "https://github.com/other/repo",
            "https://github.com.evil.example/This-null/x",
            "http://www.nexusmods.com/",
            "https://evil.example/",
            "https://www.nexusmods.com.evil.example/",
            "https://nexusmods.com@evil.example/",
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "https://www.nexusmods.com/a b",
            "https://www.nexusmods.com/\nx",
            "",
        ] {
            assert!(!allowed_url(bad), "{bad}");
        }
    }

    #[test]
    fn only_real_plain_paths_are_handed_to_explorer() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("my log.log");
        fs::write(&file, "x").unwrap();
        let shown = explorer_path(&file.to_string_lossy().replace('\\', "/")).unwrap();
        assert!(!shown.contains('/') && shown.ends_with("my log.log"));
        assert!(explorer_path("").is_none());
        assert!(explorer_path(&tmp.path().join("missing.log").to_string_lossy()).is_none());
        assert!(explorer_path("C:\\x\" /root,C:\\Windows").is_none());
    }

    #[test]
    fn a_game_folder_is_compared_without_caring_about_slashes_or_case() {
        assert!(same_dir("D:\\Games\\Cyberpunk 2077", "d:/games/cyberpunk 2077/"));
        assert!(!same_dir("D:\\Games\\Cyberpunk 2077", "E:\\Games\\Cyberpunk 2077"));
    }

    #[test]
    fn the_install_id_is_made_once_and_kept_with_the_other_settings() {
        let tmp = tempfile::tempdir().unwrap();
        save_settings(tmp.path(), &Settings { game_dir: Some("D:\\g".into()), ..Default::default() }).unwrap();
        let a = install_id(tmp.path()).unwrap();
        assert_eq!(a.len(), 36);
        assert_eq!(install_id(tmp.path()).unwrap(), a);
        assert_eq!(load_settings(tmp.path()).game_dir.as_deref(), Some("D:\\g"));
    }

    #[test]
    fn atomic_writes_replace_the_file_and_leave_no_temp_file() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("a.json");
        write_atomic(p.clone(), "one").unwrap();
        write_atomic(p.clone(), "two").unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), "two");
        assert!(!p.with_extension("tmp").exists());
    }

    #[test]
    fn guard_blocks_while_running() {
        assert!(matches!(guard_not_running(true), Err(AppError::GameRunning)));
        assert!(guard_not_running(false).is_ok());
    }

    #[test]
    fn tail_keeps_last_lines() {
        assert_eq!(tail_lines("a\nb\nc\nd", 2), vec!["c", "d"]);
        assert_eq!(tail_lines("a", 5), vec!["a"]);
        assert!(tail_lines("", 3).is_empty());
    }

    #[test]
    fn settings_default_when_missing_and_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load_settings(tmp.path()).game_dir.is_none());
        save_settings(
            tmp.path(),
            &Settings {
                game_dir: Some("D:\\g".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(load_settings(tmp.path()).game_dir.as_deref(), Some("D:\\g"));
    }

    #[test]
    fn language_and_theme_survive_a_restart_and_old_files_still_load() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(settings_path(tmp.path()), r#"{"game_dir":"D:\\g"}"#).unwrap();
        let old = load_settings(tmp.path());
        assert_eq!((old.locale, old.theme, old.motion, old.translate, old.updates, old.translate_lang), (None, None, None, None, None, None));
        save_settings(
            tmp.path(),
            &Settings { locale: Some("ru".into()), theme: Some("dark".into()), motion: Some("full".into()), translate: Some("on".into()), updates: Some("off".into()), translate_lang: Some("de".into()), ..Default::default() },
        )
        .unwrap();
        let back = load_settings(tmp.path());
        assert_eq!((back.locale.as_deref(), back.theme.as_deref(), back.motion.as_deref(), back.translate.as_deref(), back.updates.as_deref(), back.translate_lang.as_deref()), (Some("ru"), Some("dark"), Some("full"), Some("on"), Some("off"), Some("de")));
    }
}
