use crate::conflicts::find_conflicts;
use crate::error::AppError;
use crate::layout::{is_reserved_device, is_rooted, map_all};
use crate::meta::{self, ModManifest, PackEntry, PackManifest, MAX_IMAGE, MAX_VERSION};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const MANIFEST: &str = "launcher-mod.json";
const PACK_MANIFEST: &str = "launcher-pack.json";
const META_DIR: &str = ".meta";
const MAX_MANIFEST: u64 = 256 * 1024;
const MAX_README: u64 = 64 * 1024;
const MAX_PACK: usize = 200;
const MAX_TEXT: usize = 4000;
const BACKUP_DIR: &str = ".overwritten";

static STATE_LOCK: Mutex<()> = Mutex::new(());
static IGNORED: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn take_ignored() -> Vec<String> {
    std::mem::take(&mut *IGNORED.lock().unwrap_or_else(|e| e.into_inner()))
}
static BUSY_NAMES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn root_key(root: &Path) -> String {
    root.to_string_lossy().to_ascii_lowercase()
}

fn lock() -> MutexGuard<'static, ()> {
    STATE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Limits {
    pub max_entries: usize,
    pub max_total: u64,
    pub max_file: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_entries: 50_000,
            max_total: 10 * 1024 * 1024 * 1024,
            max_file: 4 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Backup {
    pub file: String,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModRecord {
    pub name: String,
    pub enabled: bool,
    pub files: Vec<String>,
    pub installed_files: Vec<String>,
    pub created_dirs: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub nexus_id: Option<u64>,
    #[serde(default)]
    pub backups: Vec<Backup>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct ApplyReport {
    pub enabled: Vec<String>,
    pub disabled: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub missing: Vec<String>,
}

pub struct Library {
    root: PathBuf,
    game: PathBuf,
}

struct Plan {
    name: String,
    files: Vec<(usize, String)>,
    cover: Option<(usize, &'static str)>,
    description: String,
    version: String,
    nexus_id: Option<u64>,
    bytes: u64,
}

struct Reservation {
    root_key: String,
    names: Vec<String>,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        let mut busy = BUSY_NAMES.lock().unwrap_or_else(|e| e.into_inner());
        busy.retain(|(r, b)| !(*r == self.root_key && self.names.iter().any(|n| n.eq_ignore_ascii_case(b))));
    }
}

fn validate_name(name: &str) -> Result<(), AppError> {
    let bad = name.is_empty()
        || name.len() > 64
        || name.starts_with('.')
        || name.ends_with('.')
        || name.trim() != name
        || name.chars().any(|c| c.is_control())
        || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        || is_reserved_device(name)
        || name.eq_ignore_ascii_case("state.json")
        || name.eq_ignore_ascii_case("state.json.tmp");
    if bad {
        Err(AppError::InvalidName(name.to_string()))
    } else {
        Ok(())
    }
}

fn sanitize_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let short: String = cleaned
        .trim()
        .trim_start_matches(['.', ' '])
        .chars()
        .take(56)
        .collect::<String>();
    let short = short.trim().trim_end_matches(['.', ' ']).to_string();
    if is_reserved_device(&short) || short.eq_ignore_ascii_case("state.json") || short.eq_ignore_ascii_case("state.json.tmp") {
        format!("_{short}")
    } else {
        short
    }
}

fn unique_name(taken: &[String], base: &str) -> String {
    let exists = |n: &str| taken.iter().any(|t| t.eq_ignore_ascii_case(n));
    if !exists(base) {
        return base.to_string();
    }
    (2..1000)
        .map(|i| format!("{base} ({i})"))
        .find(|n| !exists(n))
        .unwrap_or_else(|| format!("{base} (copy)"))
}

fn valid_folder(folder: &str) -> bool {
    !folder.is_empty() && folder.len() <= 16 && folder.chars().all(|c| c.is_ascii_alphanumeric())
}

fn rel_string(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn list_entries(
    archive: &mut ZipArchive<fs::File>,
    prefix: &str,
) -> Result<Vec<(usize, String, u64)>, AppError> {
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let norm = entry.name().replace('\\', "/");
        if !prefix.is_empty() && !norm.starts_with(prefix) {
            continue;
        }
        out.push((i, norm[prefix.len()..].to_string(), entry.size()));
    }
    Ok(out)
}

fn read_bytes(archive: &mut ZipArchive<fs::File>, index: usize, cap: u64) -> Result<Vec<u8>, AppError> {
    let mut entry = archive.by_index(index)?;
    let mut buf = Vec::new();
    entry.by_ref().take(cap).read_to_end(&mut buf)?;
    Ok(buf)
}

fn same_mod_name(a: &str, b: &str) -> bool {
    fn plain(s: &str) -> String {
        let lower = s.trim().to_ascii_lowercase();
        match lower.rfind(" (") {
            Some(i) if lower.ends_with(')') && lower[i + 2..lower.len() - 1].chars().all(|c| c.is_ascii_digit()) && lower.len() - 1 > i + 2 => {
                lower[..i].trim_end().to_string()
            }
            _ => lower,
        }
    }
    plain(a) == plain(b)
}

fn read_json<T: DeserializeOwned>(
    archive: &mut ZipArchive<fs::File>,
    entries: &[(usize, String, u64)],
    file: &str,
) -> Option<T> {
    let (index, _, _) = entries.iter().find(|(_, rel, _)| rel.eq_ignore_ascii_case(file))?;
    let bytes = read_bytes(archive, *index, MAX_MANIFEST).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn plan_single(
    archive: &mut ZipArchive<fs::File>,
    prefix: &str,
    name: &str,
    manifest: &ModManifest,
    fallback_version: &str,
    limits: &Limits,
) -> Result<Plan, AppError> {
    let list = list_entries(archive, prefix)?;
    let mut files: Vec<(usize, String)> = Vec::new();
    let mut bytes = 0u64;
    let rels: Vec<&str> = list.iter().map(|(_, rel, _)| rel.as_str()).collect();
    let (mapped, ignored) = map_all(&rels, name)?;
    for ((i, _, size), target) in list.iter().zip(mapped) {
        if let Some(p) = target {
            let r = rel_string(&p);
            if !files.iter().any(|(_, x)| x.eq_ignore_ascii_case(&r)) {
                if *size > limits.max_file {
                    return Err(AppError::Zip("a file in the archive is too large".into()));
                }
                bytes = bytes.saturating_add(*size);
                if bytes > limits.max_total {
                    return Err(AppError::Zip("the archive is too large".into()));
                }
                files.push((*i, r));
            }
        }
    }
    if files.is_empty() {
        if let Some(first) = ignored.first() {
            return Err(AppError::Unmappable(first.clone()));
        }
        let seen: Vec<&str> = list.iter().map(|(_, rel, _)| rel.as_str()).take(6).collect();
        return Err(AppError::NoFiles(seen.join(", ")));
    }
    IGNORED.lock().unwrap_or_else(|e| e.into_inner()).extend(ignored);

    let images: Vec<(usize, &str, u64)> = list
        .iter()
        .filter(|(_, rel, size)| meta::is_image(rel) && !is_rooted(rel) && *size > 0 && *size <= MAX_IMAGE)
        .map(|(i, rel, size)| (*i, rel.as_str(), *size))
        .collect();
    let named = manifest
        .cover
        .as_deref()
        .and_then(|c| images.iter().position(|(_, r, _)| r.eq_ignore_ascii_case(c)));
    let picked = named.or_else(|| {
        let c: Vec<(String, u64)> = images.iter().map(|(_, r, s)| (r.to_string(), *s)).collect();
        meta::pick_cover(&c)
    });
    let cover = picked.and_then(|k| {
        let (i, r, _) = images[k];
        meta::image_ext(r).map(|e| (i, e))
    });

    let description = if !manifest.description.trim().is_empty() {
        manifest.description.trim().chars().take(MAX_TEXT).collect()
    } else {
        let docs: Vec<(usize, String)> = list
            .iter()
            .filter(|(_, rel, _)| !is_rooted(rel))
            .map(|(i, rel, _)| (*i, rel.clone()))
            .collect();
        let names: Vec<String> = docs.iter().map(|(_, n)| n.clone()).collect();
        match meta::pick_readme(&names) {
            Some(k) => {
                let bytes = read_bytes(archive, docs[k].0, MAX_README)?;
                meta::clean_description(&String::from_utf8_lossy(&bytes))
            }
            None => String::new(),
        }
    };

    let declared = manifest.version.trim();
    let version = if declared.is_empty() {
        meta::version_from_name(fallback_version)
    } else {
        declared.chars().take(MAX_VERSION).collect()
    };

    Ok(Plan {
        name: name.to_string(),
        files,
        cover,
        description,
        version,
        nexus_id: None,
        bytes,
    })
}

fn extract(
    archive: &mut ZipArchive<fs::File>,
    plan: &Plan,
    dest_root: &Path,
    limits: &Limits,
) -> Result<(), AppError> {
    let mut total = 0u64;
    for (i, rel) in &plan.files {
        let mut entry = archive.by_index(*i)?;
        let declared = entry.size();
        if declared > limits.max_file {
            return Err(AppError::Zip("a file in the archive is too large".into()));
        }
        total = total.saturating_add(declared);
        if total > limits.max_total {
            return Err(AppError::Zip("the archive is too large".into()));
        }
        let dest = dest_root.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&dest)?;
        let copied = io::copy(&mut entry.by_ref().take(declared), &mut out)?;
        let mut probe = [0u8; 1];
        if copied != declared || entry.read(&mut probe)? > 0 {
            return Err(AppError::Zip("an entry does not match its announced size".into()));
        }
    }
    if let Some((i, ext)) = plan.cover {
        let bytes = read_bytes(archive, i, MAX_IMAGE)?;
        let dir = dest_root.join(META_DIR);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join(format!("cover.{ext}")), bytes)?;
    }
    Ok(())
}

fn zip_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, AppError> {
    serde_json::to_vec_pretty(value).map_err(|e| AppError::State(e.to_string()))
}

impl Library {
    pub fn new(root: PathBuf, game: PathBuf) -> Library {
        Library { root, game }
    }

    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    fn load(&self) -> Result<Vec<ModRecord>, AppError> {
        match fs::read_to_string(self.state_path()) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| AppError::State(e.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    fn save(&self, records: &[ModRecord]) -> Result<(), AppError> {
        fs::create_dir_all(&self.root)?;
        let text =
            serde_json::to_string_pretty(records).map_err(|e| AppError::State(e.to_string()))?;
        let tmp = self.root.join("state.json.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, self.state_path())?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<ModRecord>, AppError> {
        self.load()
    }

    pub fn game_dir(&self) -> &Path {
        &self.game
    }

    pub fn install_any(&self, zip_path: &Path, suggested: &str) -> Result<Vec<ModRecord>, AppError> {
        self.install_any_with(zip_path, suggested, &Limits::default())
    }

    fn reserve(&self, bases: &[String]) -> Result<Reservation, AppError> {
        let _g = lock();
        let records = self.load()?;
        let key = root_key(&self.root);
        let mut busy = BUSY_NAMES.lock().unwrap_or_else(|e| e.into_inner());
        let mut taken: Vec<String> = records
            .iter()
            .map(|r| r.name.clone())
            .chain(busy.iter().filter(|(r, _)| *r == key).map(|(_, n)| n.clone()))
            .collect();
        fs::create_dir_all(&self.root)?;
        let mut names: Vec<String> = Vec::new();
        for base in bases {
            let mut candidate = unique_name(&taken, base);
            loop {
                let dir = self.root.join(&candidate);
                if !dir.exists() || (dir.is_dir() && fs::remove_dir_all(&dir).is_ok()) {
                    break;
                }
                taken.push(candidate.clone());
                candidate = unique_name(&taken, base);
            }
            taken.push(candidate.clone());
            names.push(candidate);
        }
        for (i, name) in names.iter().enumerate() {
            if let Err(e) = fs::create_dir(self.root.join(name)) {
                for done in &names[..i] {
                    let _ = fs::remove_dir(self.root.join(done));
                }
                return Err(e.into());
            }
        }
        busy.extend(names.iter().map(|n| (key.clone(), n.clone())));
        Ok(Reservation { root_key: key, names })
    }

    pub fn install_any_with(
        &self,
        zip_path: &Path,
        suggested: &str,
        limits: &Limits,
    ) -> Result<Vec<ModRecord>, AppError> {
        self.install_report_with(zip_path, suggested, limits).map(|r| r.0)
    }

    pub fn install_report(&self, zip_path: &Path, suggested: &str) -> Result<(Vec<ModRecord>, Vec<String>), AppError> {
        self.install_report_with(zip_path, suggested, &Limits::default())
    }

    pub fn install_report_with(
        &self,
        zip_path: &Path,
        suggested: &str,
        limits: &Limits,
    ) -> Result<(Vec<ModRecord>, Vec<String>), AppError> {
        take_ignored();
        let mut archive = ZipArchive::new(fs::File::open(zip_path)?)?;
        if archive.len() > limits.max_entries {
            return Err(AppError::Zip("the archive has too many entries".into()));
        }
        let stem = zip_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let root_list = list_entries(&mut archive, "")?;
        let pack: Option<PackManifest> = read_json(&mut archive, &root_list, PACK_MANIFEST);
        let single = pack.is_none();
        let mut jobs: Vec<(String, String, ModManifest)> = Vec::new();

        match pack {
            Some(pack) => {
                if pack.mods.is_empty() || pack.mods.len() > MAX_PACK {
                    return Err(AppError::Unmappable("invalid mod pack".into()));
                }
                for entry in &pack.mods {
                    if !valid_folder(&entry.folder) {
                        return Err(AppError::UnsafePath(entry.folder.clone()));
                    }
                    let cleaned = sanitize_name(&entry.name);
                    let base = if cleaned.is_empty() { entry.folder.clone() } else { cleaned };
                    jobs.push((base, format!("mods/{}/", entry.folder), ModManifest::from(entry)));
                }
            }
            None => {
                validate_name(suggested)?;
                let manifest: ModManifest = read_json(&mut archive, &root_list, MANIFEST).unwrap_or_default();
                let cleaned = sanitize_name(&manifest.name);
                let base = if cleaned.is_empty() { suggested.to_string() } else { cleaned };
                jobs.push((base, String::new(), manifest));
            }
        }

        let mut skipped: Vec<String> = Vec::new();
        if !single {
            let existing = self.load()?;
            jobs.retain(|(name, _, manifest)| {
                let dup = existing
                    .iter()
                    .any(|r| same_mod_name(&r.name, name) && r.version.trim() == manifest.version.trim());
                if dup {
                    skipped.push(name.clone());
                }
                !dup
            });
            if jobs.is_empty() {
                return Ok((Vec::new(), skipped));
            }
        }

        let bases: Vec<String> = jobs.iter().map(|j| j.0.clone()).collect();
        let reservation = self.reserve(&bases)?;
        let cleanup = |names: &[String]| {
            for n in names {
                let _ = fs::remove_dir_all(self.root.join(n));
            }
        };

        let built = (|| -> Result<Vec<Plan>, AppError> {
            let mut plans: Vec<Plan> = Vec::new();
            for (name, (_, prefix, manifest)) in reservation.names.iter().zip(&jobs) {
                let fallback = if single { stem.as_str() } else { "" };
                let mut plan = plan_single(&mut archive, prefix, name, manifest, fallback, limits)?;
                if single {
                    if let Some(n) = crate::nexus::parse_filename(&stem) {
                        plan.nexus_id = Some(n.mod_id);
                        if plan.version.is_empty() {
                            plan.version = n.version;
                        }
                    }
                }
                plans.push(plan);
            }
            if plans.iter().fold(0u64, |a, p| a.saturating_add(p.bytes)) > limits.max_total {
                return Err(AppError::Zip("the archive is too large".into()));
            }
            for plan in &plans {
                extract(&mut archive, plan, &self.root.join(&plan.name), limits)?;
            }
            Ok(plans)
        })();
        let plans = match built {
            Ok(p) => p,
            Err(e) => {
                cleanup(&reservation.names);
                return Err(e);
            }
        };

        let added: Vec<ModRecord> = plans
            .into_iter()
            .map(|p| ModRecord {
                name: p.name,
                files: p.files.into_iter().map(|(_, r)| r).collect(),
                description: p.description,
                version: p.version,
                image: p.cover.map(|(_, e)| format!("cover.{e}")),
                nexus_id: p.nexus_id,
                ..ModRecord::default()
            })
            .collect();
        {
            let _g = lock();
            let committed = self.load().and_then(|mut records| {
                records.extend(added.clone());
                self.save(&records)
            });
            if let Err(e) = committed {
                cleanup(&reservation.names);
                return Err(e);
            }
        }
        Ok((added, skipped))
    }

    pub fn adopt(&self, name: &str, files: &[String]) -> Result<ModRecord, AppError> {
        validate_name(name)?;
        let _g = lock();
        let mut records = self.load()?;
        let key = root_key(&self.root);
        let busy = BUSY_NAMES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|(r, b)| *r == key && b.eq_ignore_ascii_case(name));
        if busy || records.iter().any(|r| r.name.eq_ignore_ascii_case(name)) {
            return Err(AppError::InvalidName(format!("{name} is already installed")));
        }
        let dest_root = self.root.join(name);
        if dest_root.exists() {
            fs::remove_dir_all(&dest_root)?;
        }
        let copied = (|| -> Result<(), AppError> {
            for rel in files {
                let dst = dest_root.join(rel);
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(self.game.join(rel), &dst)?;
            }
            Ok(())
        })();
        if let Err(e) = copied {
            let _ = fs::remove_dir_all(&dest_root);
            return Err(e);
        }
        let record = ModRecord {
            name: name.to_string(),
            enabled: true,
            files: files.to_vec(),
            installed_files: files.to_vec(),
            ..ModRecord::default()
        };
        records.push(record.clone());
        if let Err(e) = self.save(&records) {
            let _ = fs::remove_dir_all(&dest_root);
            return Err(e);
        }
        Ok(record)
    }

    pub fn set_info(
        &self,
        name: &str,
        description: &str,
        version: &str,
        image_src: Option<&Path>,
    ) -> Result<ModRecord, AppError> {
        let _g = lock();
        let mut records = self.load()?;
        let idx = records
            .iter()
            .position(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        if let Some(src) = image_src {
            let ext = src
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(meta::image_ext)
                .ok_or_else(|| AppError::Unmappable("the image must be png, jpg or webp".into()))?;
            let len = fs::metadata(src)?.len();
            if len == 0 || len > MAX_IMAGE {
                return Err(AppError::Unmappable("the image is empty or too large".into()));
            }
            let bytes = fs::read(src)?;
            let dir = self.root.join(name).join(META_DIR);
            fs::create_dir_all(&dir)?;
            for old in ["png", "jpg", "webp"] {
                let _ = fs::remove_file(dir.join(format!("cover.{old}")));
            }
            fs::write(dir.join(format!("cover.{ext}")), bytes)?;
            records[idx].image = Some(format!("cover.{ext}"));
        }
        records[idx].description = description.trim().chars().take(MAX_TEXT).collect();
        records[idx].version = version.trim().chars().take(MAX_VERSION).collect();
        self.save(&records)?;
        Ok(records[idx].clone())
    }

    pub fn image_data_url(&self, name: &str) -> Result<Option<String>, AppError> {
        let records = self.load()?;
        let rec = records
            .iter()
            .find(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        let Some(img) = &rec.image else {
            return Ok(None);
        };
        if img.contains(['/', '\\']) {
            return Ok(None);
        }
        match fs::read(self.root.join(name).join(META_DIR).join(img)) {
            Ok(bytes) => {
                let ext = img.rsplit('.').next().unwrap_or("png");
                Ok(Some(format!("data:{};base64,{}", meta::mime_for(ext), STANDARD.encode(bytes))))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn write_mod(
        &self,
        zw: &mut ZipWriter<fs::File>,
        record: &ModRecord,
        prefix: &str,
        tick: &mut dyn FnMut(),
    ) -> Result<(String, Option<String>), AppError> {
        let opts = zip_options();
        for rel in &record.files {
            zw.start_file(format!("{prefix}{rel}"), opts)?;
            let mut f = fs::File::open(self.root.join(&record.name).join(rel))?;
            io::copy(&mut f, zw)?;
            tick();
        }
        let mut cover_name = None;
        if let Some(img) = &record.image {
            let path = self.root.join(&record.name).join(META_DIR).join(img);
            if !img.contains(['/', '\\']) && path.is_file() {
                let ext = img.rsplit('.').next().unwrap_or("png");
                let name = format!("launcher-cover.{ext}");
                zw.start_file(format!("{prefix}{name}"), opts)?;
                io::copy(&mut fs::File::open(path)?, zw)?;
                cover_name = Some(name);
            }
        }
        Ok((record.name.clone(), cover_name))
    }

    pub fn export_mod(&self, name: &str, dest: &Path) -> Result<(), AppError> {
        self.export_mod_with(name, dest, &mut |_, _| {})
    }

    pub fn export_mod_with(&self, name: &str, dest: &Path, progress: &mut dyn FnMut(u32, u32)) -> Result<(), AppError> {
        let records = self.load()?;
        let rec = records
            .iter()
            .find(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        let written = (|| -> Result<(), AppError> {
            let mut zw = ZipWriter::new(fs::File::create(dest)?);
            let total = rec.files.len() as u32;
            let mut done = 0;
            progress(0, total);
            let (_, cover) = self.write_mod(&mut zw, rec, "", &mut || {
                done += 1;
                progress(done, total);
            })?;
            let manifest = ModManifest {
                format: 1,
                name: rec.name.clone(),
                version: rec.version.clone(),
                description: rec.description.clone(),
                cover,
            };
            zw.start_file(MANIFEST, zip_options())?;
            zw.write_all(&json_bytes(&manifest)?)?;
            zw.finish()?;
            Ok(())
        })();
        if written.is_err() {
            let _ = fs::remove_file(dest);
        }
        written
    }

    pub fn export_pack(&self, names: &[String], dest: &Path) -> Result<(), AppError> {
        self.export_pack_with(names, dest, &mut |_, _| {})
    }

    pub fn export_pack_with(&self, names: &[String], dest: &Path, progress: &mut dyn FnMut(u32, u32)) -> Result<(), AppError> {
        let records = self.load()?;
        let mut chosen: Vec<&ModRecord> = Vec::new();
        for name in names {
            chosen.push(
                records
                    .iter()
                    .find(|r| &r.name == name)
                    .ok_or_else(|| AppError::NotFound(name.clone()))?,
            );
        }
        if chosen.is_empty() || chosen.len() > MAX_PACK {
            return Err(AppError::NotFound("no mods to export".into()));
        }
        let written = (|| -> Result<(), AppError> {
            let mut zw = ZipWriter::new(fs::File::create(dest)?);
            let mut entries = Vec::new();
            let total = chosen.iter().map(|r| r.files.len() as u32).sum::<u32>();
            let mut done = 0;
            progress(0, total);
            for (i, rec) in chosen.iter().enumerate() {
                let folder = format!("m{}", i + 1);
                let (_, cover) = self.write_mod(&mut zw, rec, &format!("mods/{folder}/"), &mut || {
                    done += 1;
                    progress(done, total);
                })?;
                entries.push(PackEntry {
                    folder,
                    name: rec.name.clone(),
                    version: rec.version.clone(),
                    description: rec.description.clone(),
                    cover,
                });
            }
            zw.start_file(PACK_MANIFEST, zip_options())?;
            zw.write_all(&json_bytes(&PackManifest { format: 1, mods: entries })?)?;
            zw.finish()?;
            Ok(())
        })();
        if written.is_err() {
            let _ = fs::remove_file(dest);
        }
        written
    }

    pub fn set_nexus_id(&self, name: &str, nexus_id: Option<u64>) -> Result<ModRecord, AppError> {
        let _g = lock();
        let mut records = self.load()?;
        let rec = records
            .iter_mut()
            .find(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        rec.nexus_id = nexus_id;
        let out = rec.clone();
        self.save(&records)?;
        Ok(out)
    }

    pub fn tag_record(&self, name: &str, source: &str, version: &str, description: &str) -> Result<(), AppError> {
        let _g = lock();
        let mut records = self.load()?;
        let rec = records
            .iter_mut()
            .find(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        rec.source = Some(source.to_string());
        rec.version = version.chars().take(MAX_VERSION).collect();
        if !description.trim().is_empty() {
            rec.description = description.trim().chars().take(MAX_TEXT).collect();
        }
        self.save(&records)
    }

    pub fn activate_exclusive(&self, name: &str, force: bool) -> Result<(), AppError> {
        let _g = lock();
        let records = self.load()?;
        let target = records
            .iter()
            .find(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        if target.enabled {
            return Ok(());
        }
        let same_mod = |r: &ModRecord| {
            let tagged = target.source.is_some() && r.source == target.source;
            let nexus = target.nexus_id.is_some_and(|id| id > 0) && r.nexus_id == target.nexus_id;
            tagged || nexus
        };
        let others: Vec<String> = records
            .iter()
            .filter(|r| r.enabled && r.name != name && same_mod(r))
            .map(|r| r.name.clone())
            .collect();
        let conflicts: Vec<_> = find_conflicts(&self.game, &records, target)
            .into_iter()
            .filter(|c| !c.owner.as_ref().is_some_and(|o| others.contains(o)))
            .collect();
        if !conflicts.is_empty() && !force {
            return Err(AppError::Conflict(conflicts));
        }
        let mut turned_off: Vec<String> = Vec::new();
        for other in &others {
            if let Err(e) = self.disable_locked(other) {
                for back in &turned_off {
                    let _ = self.enable_locked(back, true);
                }
                return Err(e);
            }
            turned_off.push(other.clone());
        }
        match self.enable_locked(name, true) {
            Ok(()) => Ok(()),
            Err(e) => {
                for back in &turned_off {
                    let _ = self.enable_locked(back, true);
                }
                Err(e)
            }
        }
    }

    pub fn apply_preset(&self, wanted: &[String]) -> Result<ApplyReport, AppError> {
        let records = self.load()?;
        let preset = crate::presets::Preset { name: String::new(), mods: wanted.to_vec() };
        let plan = crate::presets::plan(&preset, &records);
        let mut report = ApplyReport { missing: plan.missing, ..ApplyReport::default() };
        for name in &plan.disable {
            match self.disable(name) {
                Ok(()) => report.disabled.push(name.clone()),
                Err(e) => report.failed.push((name.clone(), e.code().to_string())),
            }
        }
        for name in &plan.enable {
            match self.activate_exclusive(name, false) {
                Ok(()) => report.enabled.push(name.clone()),
                Err(e) => report.failed.push((name.clone(), e.code().to_string())),
            }
        }
        Ok(report)
    }

    fn rename(&self, from: &str, to: &str) -> Result<(), AppError> {
        let _g = lock();
        let mut records = self.load()?;
        if records.iter().any(|r| r.name == to) {
            return Ok(());
        }
        let idx = records
            .iter()
            .position(|r| r.name == from)
            .ok_or_else(|| AppError::NotFound(from.to_string()))?;
        fs::rename(self.root.join(from), self.root.join(to))?;
        let (old_bk, new_bk) = (self.backup_root(from), self.backup_root(to));
        if old_bk.exists() && fs::rename(&old_bk, &new_bk).is_err() {
            let _ = fs::rename(self.root.join(to), self.root.join(from));
            return Ok(());
        }
        records[idx].name = to.to_string();
        if let Err(e) = self.save(&records) {
            let _ = fs::rename(self.root.join(to), self.root.join(from));
            let _ = fs::rename(&new_bk, &old_bk);
            return Err(e);
        }
        Ok(())
    }

    pub fn replace_older(&self, names: &[String]) -> Vec<String> {
        let mut replaced = Vec::new();
        for name in names {
            let mut outgoing: Vec<String> = Vec::new();
            let wanted = self.load().is_ok_and(|records| {
                let Some(target) = records.iter().find(|r| &r.name == name) else {
                    return false;
                };
                if target.enabled || !target.nexus_id.is_some_and(|id| id > 0) {
                    return false;
                }
                outgoing = records
                    .iter()
                    .filter(|r| {
                        r.enabled
                            && &r.name != name
                            && r.nexus_id == target.nexus_id
                            && same_mod_name(&r.name, name)
                            && matches!(
                                crate::nexus::compare_versions(&r.version, &target.version),
                                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                            )
                    })
                    .map(|r| r.name.clone())
                    .collect();
                records.iter().any(|r| r.enabled && &r.name != name && r.nexus_id == target.nexus_id)
            });
            if wanted && self.activate_exclusive(name, false).is_ok() {
                replaced.push(name.clone());
                let mut current = name.clone();
                for old in &outgoing {
                    if self.uninstall(old).is_ok() && self.rename(&current, old).is_ok() && self.load().is_ok_and(|r| r.iter().any(|x| &x.name == old)) {
                        current = old.clone();
                    }
                }
            }
        }
        replaced
    }

    pub fn enable(&self, name: &str, force: bool) -> Result<(), AppError> {
        let _g = lock();
        self.enable_locked(name, force)
    }

    fn backup_root(&self, name: &str) -> PathBuf {
        self.root.join(BACKUP_DIR).join(name)
    }

    fn rollback(&self, written: &[String], backups: &[Backup], created: &[String], backup_root: &Path) {
        for rel in written {
            let dst = self.game.join(rel);
            let saved = backups.iter().find(|b| &b.file == rel).map(|b| backup_root.join(&b.file));
            match saved {
                Some(bak) if bak.is_file() => {
                    let _ = fs::copy(&bak, &dst);
                }
                _ => {
                    let _ = fs::remove_file(&dst);
                }
            }
        }
        self.remove_files_and_dirs(&[], created);
        let _ = fs::remove_dir_all(backup_root);
    }

    fn enable_locked(&self, name: &str, force: bool) -> Result<(), AppError> {
        let mut records = self.load()?;
        let idx = records
            .iter()
            .position(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        if records[idx].enabled {
            return Ok(());
        }
        let candidate = records[idx].clone();
        let conflicts = find_conflicts(&self.game, &records, &candidate);
        if !conflicts.is_empty() && !force {
            return Err(AppError::Conflict(conflicts));
        }
        let backup_root = self.backup_root(name);
        let mut written: Vec<String> = Vec::new();
        let mut backups: Vec<Backup> = Vec::new();
        let mut created: Vec<String> = Vec::new();
        let copy_all = (|| -> Result<(), AppError> {
            for rel in &candidate.files {
                let src = self.root.join(name).join(rel);
                let dst = self.game.join(rel);
                if let Some(parent) = dst.parent() {
                    let mut missing = Vec::new();
                    let mut cur = parent.to_path_buf();
                    while !cur.exists() && cur != self.game {
                        missing.push(cur.clone());
                        if !cur.pop() {
                            break;
                        }
                    }
                    fs::create_dir_all(parent)?;
                    for dir in missing {
                        if let Ok(r) = dir.strip_prefix(&self.game) {
                            let r = rel_string(r);
                            if !created.contains(&r) {
                                created.push(r);
                            }
                        }
                    }
                }
                if dst.is_file() {
                    let owner = records
                        .iter()
                        .find(|r| {
                            r.name != name && r.enabled && r.installed_files.iter().any(|f| f.eq_ignore_ascii_case(rel))
                        })
                        .map(|r| r.name.clone());
                    let bak = backup_root.join(rel);
                    if let Some(parent) = bak.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(&dst, &bak)?;
                    backups.push(Backup {
                        file: rel.clone(),
                        owner,
                    });
                }
                fs::copy(&src, &dst)?;
                written.push(rel.clone());
            }
            Ok(())
        })();
        if let Err(e) = copy_all {
            self.rollback(&written, &backups, &created, &backup_root);
            return Err(e);
        }
        for b in &backups {
            if let Some(owner) = &b.owner {
                if let Some(other) = records.iter_mut().find(|r| &r.name == owner) {
                    other.installed_files.retain(|f| !f.eq_ignore_ascii_case(&b.file));
                }
            }
        }
        records[idx].enabled = true;
        records[idx].installed_files = written.clone();
        records[idx].created_dirs = created.clone();
        records[idx].backups = backups.clone();
        if let Err(e) = self.save(&records) {
            self.rollback(&written, &backups, &created, &backup_root);
            return Err(e);
        }
        Ok(())
    }

    fn remove_files_and_dirs(&self, files: &[String], dirs: &[String]) {
        for f in files {
            let _ = fs::remove_file(self.game.join(f));
        }
        let mut dirs = dirs.to_vec();
        dirs.sort_by_key(|d| std::cmp::Reverse(d.len()));
        for d in dirs {
            let _ = fs::remove_dir(self.game.join(d));
        }
    }

    fn sync_back(game_file: &Path, library_file: &Path) {
        let (Ok(g), Ok(l)) = (fs::metadata(game_file), fs::metadata(library_file)) else {
            return;
        };
        if !g.is_file() || !l.is_file() {
            return;
        }
        let changed = g.len() != l.len()
            || match (g.modified(), l.modified()) {
                (Ok(a), Ok(b)) => a != b,
                _ => false,
            };
        if changed {
            let _ = fs::copy(game_file, library_file);
        }
    }

    pub fn disable(&self, name: &str) -> Result<(), AppError> {
        let _g = lock();
        self.disable_locked(name)
    }

    fn only_junk(dir: &Path) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        entries.flatten().all(|e| {
            let p = e.path();
            let is_log = |p: &Path| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("log"));
            if p.is_dir() {
                p.file_name().is_some_and(|n| n.eq_ignore_ascii_case("logs")) && Self::only_junk(&p)
            } else {
                is_log(&p)
            }
        })
    }

    fn sweep_up(&self, files: &[String]) {
        for f in files {
            let mut parts: Vec<&str> = f.split('/').collect();
            parts.pop();
            let mut cleared = false;
            while parts.len() >= 2 {
                let dir = self.game.join(parts.join("/"));
                let has_log = fs::read_dir(&dir).is_ok_and(|mut d| d.any(|e| e.is_ok()));
                if !Self::only_junk(&dir) || !(has_log || cleared) {
                    break;
                }
                if fs::remove_dir_all(&dir).is_err() {
                    break;
                }
                cleared = true;
                parts.pop();
            }
        }
    }

    fn disable_locked(&self, name: &str) -> Result<(), AppError> {
        self.disable_inner(name, false)
    }

    fn disable_inner(&self, name: &str, purge_logs: bool) -> Result<(), AppError> {
        let mut records = self.load()?;
        let idx = records
            .iter()
            .position(|r| r.name == name)
            .ok_or_else(|| AppError::NotFound(name.to_string()))?;
        if !records[idx].enabled {
            return Ok(());
        }
        let library_dir = self.root.join(name);
        let mut stuck: Vec<String> = Vec::new();
        let mut removed: Vec<String> = Vec::new();
        for f in records[idx].installed_files.clone() {
            let game_file = self.game.join(&f);
            Self::sync_back(&game_file, &library_dir.join(&f));
            match fs::remove_file(&game_file) {
                Ok(()) => removed.push(f),
                Err(e) if e.kind() == io::ErrorKind::NotFound => removed.push(f),
                Err(_) => stuck.push(f),
            }
        }
        if !stuck.is_empty() {
            records[idx].installed_files = stuck.clone();
            self.save(&records)?;
            let names: Vec<String> = stuck
                .iter()
                .take(3)
                .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
                .collect();
            return Err(AppError::FilesInUse(names.join(", ")));
        }
        let backup_root = self.backup_root(name);
        for b in records[idx].backups.clone() {
            let bak = backup_root.join(&b.file);
            if !bak.is_file() {
                continue;
            }
            let owner_alive = match &b.owner {
                None => true,
                Some(o) => records.iter().any(|r| &r.name == o && r.enabled),
            };
            if !owner_alive {
                continue;
            }
            let dst = self.game.join(&b.file);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&bak, &dst)?;
            if let Some(o) = &b.owner {
                if let Some(other) = records.iter_mut().find(|r| &r.name == o) {
                    if !other.installed_files.iter().any(|f| f.eq_ignore_ascii_case(&b.file)) {
                        other.installed_files.push(b.file.clone());
                    }
                }
            }
        }
        let _ = fs::remove_dir_all(&backup_root);
        let dirs = records[idx].created_dirs.clone();
        self.remove_files_and_dirs(&[], &dirs);
        if purge_logs {
            self.sweep_up(&removed);
        }
        records[idx].enabled = false;
        records[idx].installed_files.clear();
        records[idx].created_dirs.clear();
        records[idx].backups.clear();
        self.save(&records)
    }

    pub fn uninstall(&self, name: &str) -> Result<(), AppError> {
        let _g = lock();
        self.disable_inner(name, true)?;
        let mut records = self.load()?;
        records.retain(|r| r.name != name);
        self.save(&records)?;
        let _ = fs::remove_dir_all(self.root.join(name));
        let _ = fs::remove_dir_all(self.backup_root(name));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflicts::Conflict;

    fn make_zip(dir: &Path, file: &str, entries: &[(&str, &str)]) -> PathBuf {
        let path = dir.join(file);
        let mut zw = ZipWriter::new(fs::File::create(&path).unwrap());
        let opts = SimpleFileOptions::default();
        for (name, body) in entries {
            zw.start_file(*name, opts).unwrap();
            zw.write_all(body.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
        path
    }

    struct Env {
        _tmp: tempfile::TempDir,
        work: PathBuf,
        game: PathBuf,
        lib: Library,
    }

    fn env() -> Env {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("work");
        let game = tmp.path().join("game");
        fs::create_dir_all(&work).unwrap();
        fs::create_dir_all(&game).unwrap();
        let lib = Library::new(tmp.path().join("lib"), game.clone());
        Env {
            _tmp: tmp,
            work,
            game,
            lib,
        }
    }

    fn inst(lib: &Library, zip: &Path, name: &str) -> Result<ModRecord, AppError> {
        lib.install_any(zip, name)?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Unmappable("nothing installed".into()))
    }

    const CET: &str = "bin/x64/plugins/cyber_engine_tweaks/mods/m/init.lua";

    #[test]
    fn common_real_world_archive_layouts_land_where_the_game_expects_them() {
        let cet = "bin/x64/plugins/cyber_engine_tweaks/mods";
        let cases: Vec<(&str, Vec<&str>, Vec<String>)> = vec![
            ("rooted cet mod", vec!["bin/x64/plugins/cyber_engine_tweaks/mods/a/init.lua"], vec![format!("{cet}/a/init.lua")]),
            ("wrapper around a rooted tree", vec!["My Mod v1/archive/pc/mod/a.archive", "My Mod v1/readme.txt"], vec!["archive/pc/mod/a.archive".into()]),
            ("double wrapper", vec!["Release/My Mod/archive/pc/mod/a.archive"], vec!["archive/pc/mod/a.archive".into()]),
            ("flat archive, xl, docs and preview", vec!["a.archive", "a.archive.xl", "README.md", "preview.png", "guide.pdf"], vec!["archive/pc/mod/a.archive".into(), "archive/pc/mod/a.archive.xl".into()]),
            ("flat redscript goes under the mod name", vec!["a.reds"], vec!["r6/scripts/m/a.reds".into()]),
            ("wrapped redscript", vec!["Pack/r6/scripts/Pack/a.reds"], vec!["r6/scripts/Pack/a.reds".into()]),
            ("bare cet folder", vec!["Cool/init.lua", "Cool/lib/x.lua"], vec![format!("{cet}/Cool/init.lua"), format!("{cet}/Cool/lib/x.lua")]),
            ("tweakxl yaml", vec!["tweaks/a.yaml", "b.tweak"], vec!["r6/tweaks/tweaks/a.yaml".into(), "r6/tweaks/b.tweak".into()]),
            ("red4ext plugin in a folder", vec!["Plug/Plug.dll"], vec!["red4ext/plugins/Plug/Plug.dll".into()]),
            ("redmod", vec!["mods/Fancy/info.json", "mods/Fancy/archives/f.archive"], vec!["mods/Fancy/archives/f.archive".into(), "mods/Fancy/info.json".into()]),
            ("backslash paths", vec![r"pack\archive\pc\mod\b.archive"], vec!["archive/pc/mod/b.archive".into()]),
            ("helper script keeps its place", vec!["archive/pc/mod/a.archive", "Cyberclean.bat"], vec!["Cyberclean.bat".into(), "archive/pc/mod/a.archive".into()]),
            ("unknown extras are left out", vec!["archive/pc/mod/a.archive", "weird.bin", "notes.docx"], vec!["archive/pc/mod/a.archive".into()]),
        ];
        for (label, entries, expected) in cases {
            let e = env();
            let list: Vec<(&str, &str)> = entries.iter().map(|n| (*n, "x")).collect();
            let zip = make_zip(&e.work, "m.zip", &list);
            let rec = inst(&e.lib, &zip, "m").unwrap_or_else(|err| panic!("{label}: {err}"));
            let mut got = rec.files.clone();
            got.sort();
            let mut want = expected;
            want.sort();
            assert_eq!(got, want, "{label}");
        }
    }

    #[test]
    fn archives_with_nothing_installable_or_an_option_installer_fail_with_a_clear_reason() {
        let e = env();
        let notes = make_zip(&e.work, "n.zip", &[("PlaceholderMod.txt", "x"), ("guide.pdf", "x")]);
        assert!(matches!(e.lib.install_any(&notes, "n"), Err(AppError::NoFiles(_))));
        let fomod = make_zip(&e.work, "f.zip", &[("fomod/ModuleConfig.xml", "<x/>"), ("Option A/archive/pc/mod/a.archive", "x")]);
        assert!(matches!(e.lib.install_any(&fomod, "f"), Err(AppError::Fomod)));
    }

    #[test]
    fn switching_presets_turns_the_right_mods_on_and_off() {
        let e = env();
        let a = make_zip(&e.work, "a.zip", &[("archive/pc/mod/a.archive", "a")]);
        let b = make_zip(&e.work, "b.zip", &[("archive/pc/mod/b.archive", "b")]);
        let c = make_zip(&e.work, "c.zip", &[("archive/pc/mod/c.archive", "c")]);
        for (z, n) in [(&a, "A"), (&b, "B"), (&c, "C")] {
            inst(&e.lib, z, n).unwrap();
        }
        e.lib.enable("A", false).unwrap();
        e.lib.enable("B", false).unwrap();
        let r = e.lib.apply_preset(&["B".to_string(), "C".to_string(), "Gone".to_string()]).unwrap();
        assert_eq!((r.enabled.as_slice(), r.disabled.as_slice()), (["C".to_string()].as_slice(), ["A".to_string()].as_slice()));
        assert_eq!(r.missing, vec!["Gone".to_string()]);
        assert!(r.failed.is_empty());
        assert!(!e.game.join("archive/pc/mod/a.archive").exists());
        assert!(e.game.join("archive/pc/mod/b.archive").exists() && e.game.join("archive/pc/mod/c.archive").exists());
        let again = e.lib.apply_preset(&["B".to_string(), "C".to_string()]).unwrap();
        assert!(again.enabled.is_empty() && again.disabled.is_empty());
    }

    #[test]
    fn a_preset_with_a_conflicting_mod_reports_it_and_still_switches_the_rest() {
        let e = env();
        let a = make_zip(&e.work, "a.zip", &[("archive/pc/mod/x.archive", "a")]);
        let b = make_zip(&e.work, "b.zip", &[("archive/pc/mod/x.archive", "b")]);
        let c = make_zip(&e.work, "c.zip", &[("archive/pc/mod/c.archive", "c")]);
        for (z, n) in [(&a, "A"), (&b, "B"), (&c, "C")] {
            inst(&e.lib, z, n).unwrap();
        }
        e.lib.enable("A", false).unwrap();
        let r = e.lib.apply_preset(&["A".to_string(), "B".to_string(), "C".to_string()]).unwrap();
        assert_eq!(r.enabled, vec!["C".to_string()]);
        assert_eq!(r.failed.len(), 1);
        assert_eq!(r.failed[0].0, "B");
        assert_eq!(fs::read_to_string(e.game.join("archive/pc/mod/x.archive")).unwrap(), "a");
    }

    #[test]
    fn a_preset_picks_the_newer_version_when_two_versions_of_one_mod_exist() {
        let e = env();
        let old = make_zip(&e.work, "old.zip", &[("archive/pc/mod/a.archive", "old")]);
        let new = make_zip(&e.work, "new.zip", &[("archive/pc/mod/a.archive", "new")]);
        let o = inst(&e.lib, &old, "Cool 1").unwrap();
        e.lib.set_nexus_id(&o.name, Some(9)).unwrap();
        let n = inst(&e.lib, &new, "Cool 2").unwrap();
        e.lib.set_nexus_id(&n.name, Some(9)).unwrap();
        e.lib.enable(&o.name, false).unwrap();
        let r = e.lib.apply_preset(&[n.name.clone()]).unwrap();
        assert_eq!((r.enabled, r.disabled), (vec![n.name.clone()], vec![o.name.clone()]));
        assert_eq!(fs::read_to_string(e.game.join("archive/pc/mod/a.archive")).unwrap(), "new");
    }

    #[test]
    fn a_new_version_of_a_nexus_mod_takes_the_place_of_the_enabled_old_one() {
        let e = env();
        let old = make_zip(&e.work, "old.zip", &[("archive/pc/mod/a.archive", "old"), ("archive/pc/mod/only_old.archive", "x")]);
        let new = make_zip(&e.work, "new.zip", &[("archive/pc/mod/a.archive", "new"), ("archive/pc/mod/b.archive", "b")]);
        let o = inst(&e.lib, &old, "Cool 1").unwrap();
        e.lib.set_nexus_id(&o.name, Some(77)).unwrap();
        e.lib.enable(&o.name, false).unwrap();
        let n = inst(&e.lib, &new, "Cool 2").unwrap();
        e.lib.set_nexus_id(&n.name, Some(77)).unwrap();
        assert_eq!(e.lib.replace_older(&[n.name.clone()]), vec![n.name.clone()]);
        assert_eq!(fs::read_to_string(e.game.join("archive/pc/mod/a.archive")).unwrap(), "new");
        assert!(e.game.join("archive/pc/mod/b.archive").exists());
        assert!(!e.game.join("archive/pc/mod/only_old.archive").exists());
        let list = e.lib.list().unwrap();
        assert!(list.iter().find(|r| r.name == n.name).unwrap().enabled);
        assert!(!list.iter().find(|r| r.name == o.name).unwrap().enabled);
    }

    #[test]
    fn an_update_removes_the_older_version_but_an_old_install_keeps_the_newer_one_disabled() {
        let e = env();
        let v1 = make_zip(&e.work, "v1.zip", &[("archive/pc/mod/a.archive", "1")]);
        let v2 = make_zip(&e.work, "v2.zip", &[("archive/pc/mod/a.archive", "2")]);
        let a = inst(&e.lib, &v1, "Cool").unwrap();
        e.lib.set_nexus_id(&a.name, Some(3)).unwrap();
        e.lib.set_info(&a.name, "", "1.0", None).unwrap();
        e.lib.enable(&a.name, false).unwrap();
        let b = inst(&e.lib, &v2, "Cool").unwrap();
        e.lib.set_nexus_id(&b.name, Some(3)).unwrap();
        e.lib.set_info(&b.name, "", "2.0", None).unwrap();
        assert_eq!(e.lib.replace_older(&[b.name.clone()]), vec![b.name.clone()]);
        let names: Vec<_> = e.lib.list().unwrap().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["Cool".to_string()]);
        assert_eq!(e.lib.list().unwrap()[0].version, "2.0");
        assert_eq!(fs::read_to_string(e.game.join("archive/pc/mod/a.archive")).unwrap(), "2");

        let old = inst(&e.lib, &v1, "Cool").unwrap();
        e.lib.set_nexus_id(&old.name, Some(3)).unwrap();
        e.lib.set_info(&old.name, "", "1.0", None).unwrap();
        e.lib.replace_older(&[old.name.clone()]);
        let list = e.lib.list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list.iter().find(|r| r.enabled).unwrap().version, "1.0");
        assert_eq!(list.iter().find(|r| !r.enabled).unwrap().version, "2.0");
    }

    #[test]
    fn an_updated_mod_keeps_the_plain_name_instead_of_a_counter() {
        let e = env();
        let v1 = make_zip(&e.work, "v1.zip", &[("archive/pc/mod/a.archive", "1")]);
        let v2 = make_zip(&e.work, "v2.zip", &[("archive/pc/mod/a.archive", "2")]);
        let a = inst(&e.lib, &v1, "Codeware").unwrap();
        e.lib.set_nexus_id(&a.name, Some(4)).unwrap();
        e.lib.set_info(&a.name, "", "1.0", None).unwrap();
        e.lib.enable(&a.name, false).unwrap();
        let b = inst(&e.lib, &v2, "Codeware").unwrap();
        assert_eq!(b.name, "Codeware (2)");
        e.lib.set_nexus_id(&b.name, Some(4)).unwrap();
        e.lib.set_info(&b.name, "", "2.0", None).unwrap();
        e.lib.replace_older(&[b.name.clone()]);
        let list = e.lib.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].name.as_str(), list[0].version.as_str(), list[0].enabled), ("Codeware", "2.0", true));
        assert!(e.lib.root.join("Codeware").is_dir() && !e.lib.root.join("Codeware (2)").exists());
        e.lib.disable("Codeware").unwrap();
        e.lib.enable("Codeware", false).unwrap();
        assert_eq!(fs::read_to_string(e.game.join("archive/pc/mod/a.archive")).unwrap(), "2");
    }

    #[test]
    fn installing_the_same_version_again_replaces_the_old_copy() {
        let e = env();
        let z = make_zip(&e.work, "z.zip", &[("archive/pc/mod/a.archive", "1")]);
        let a = inst(&e.lib, &z, "Redscript").unwrap();
        e.lib.set_nexus_id(&a.name, Some(8)).unwrap();
        e.lib.set_info(&a.name, "", "0.5.32", None).unwrap();
        e.lib.enable(&a.name, false).unwrap();
        let b = inst(&e.lib, &z, "Redscript").unwrap();
        e.lib.set_nexus_id(&b.name, Some(8)).unwrap();
        e.lib.set_info(&b.name, "", "0.5.32", None).unwrap();
        e.lib.replace_older(&[b.name.clone()]);
        let names: Vec<_> = e.lib.list().unwrap().into_iter().map(|r| r.name).collect();
        assert_eq!(names, vec!["Redscript".to_string()]);
    }

    #[test]
    fn nothing_is_swapped_when_the_old_version_was_not_in_use_or_is_another_mod() {
        let e = env();
        let a = make_zip(&e.work, "a.zip", &[("archive/pc/mod/a.archive", "a")]);
        let b = make_zip(&e.work, "b.zip", &[("archive/pc/mod/b.archive", "b")]);
        let c = make_zip(&e.work, "c.zip", &[("archive/pc/mod/c.archive", "c")]);
        let ra = inst(&e.lib, &a, "A").unwrap();
        e.lib.set_nexus_id(&ra.name, Some(1)).unwrap();
        let rb = inst(&e.lib, &b, "B").unwrap();
        e.lib.set_nexus_id(&rb.name, Some(1)).unwrap();
        assert!(e.lib.replace_older(&[rb.name.clone()]).is_empty());
        let rc = inst(&e.lib, &c, "C").unwrap();
        e.lib.set_nexus_id(&rc.name, Some(2)).unwrap();
        e.lib.enable(&ra.name, false).unwrap();
        assert!(e.lib.replace_older(&[rc.name.clone()]).is_empty());
        assert!(e.game.join("archive/pc/mod/a.archive").exists());
        assert!(e.lib.replace_older(&["missing".to_string()]).is_empty());
    }

    #[test]
    fn a_swap_that_would_overwrite_someone_elses_file_leaves_the_old_version_running() {
        let e = env();
        let old = make_zip(&e.work, "old.zip", &[("archive/pc/mod/a.archive", "old")]);
        let new = make_zip(&e.work, "new.zip", &[("archive/pc/mod/a.archive", "new"), ("archive/pc/mod/taken.archive", "new")]);
        let o = inst(&e.lib, &old, "Cool 1").unwrap();
        e.lib.set_nexus_id(&o.name, Some(5)).unwrap();
        e.lib.enable(&o.name, false).unwrap();
        fs::write(e.game.join("archive/pc/mod/taken.archive"), "someone else").unwrap();
        let n = inst(&e.lib, &new, "Cool 2").unwrap();
        e.lib.set_nexus_id(&n.name, Some(5)).unwrap();
        assert!(e.lib.replace_older(&[n.name.clone()]).is_empty());
        assert_eq!(fs::read_to_string(e.game.join("archive/pc/mod/a.archive")).unwrap(), "old");
        assert_eq!(fs::read_to_string(e.game.join("archive/pc/mod/taken.archive")).unwrap(), "someone else");
        assert!(e.lib.list().unwrap().iter().find(|r| r.name == o.name).unwrap().enabled);
    }

    #[test]
    fn uninstalling_removes_every_installed_file_and_the_plugin_logs_it_left_behind() {
        let e = env();
        let z = make_zip(
            &e.work,
            "nc.zip",
            &[
                ("bin/x64/winmm.dll", "w"),
                ("red4ext/RED4ext.dll", "r"),
                ("red4ext/plugins/NightCoop/NightCoop.dll", "n"),
            ],
        );
        let rec = inst(&e.lib, &z, "NightCoop").unwrap();
        e.lib.enable(&rec.name, false).unwrap();
        let logs = e.game.join("red4ext/plugins/NightCoop/logs");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("nightcoop.log"), "log").unwrap();
        fs::write(e.game.join("red4ext/plugins/NightCoop/crash.log"), "crash").unwrap();
        e.lib.uninstall(&rec.name).unwrap();
        assert!(!e.game.join("bin/x64/winmm.dll").exists());
        assert!(!e.game.join("red4ext/RED4ext.dll").exists());
        assert!(!e.game.join("red4ext/plugins").exists());
        assert!(e.lib.list().unwrap().is_empty());
    }

    #[test]
    fn uninstalling_keeps_files_that_are_not_logs_next_to_a_mod() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[("red4ext/plugins/Cool/Cool.dll", "c")]);
        let rec = inst(&e.lib, &z, "Cool").unwrap();
        e.lib.enable(&rec.name, false).unwrap();
        fs::write(e.game.join("red4ext/plugins/Cool/settings.json"), "mine").unwrap();
        fs::write(e.game.join("red4ext/plugins/Cool/cool.log"), "log").unwrap();
        e.lib.uninstall(&rec.name).unwrap();
        assert!(e.game.join("red4ext/plugins/Cool/settings.json").exists());
        assert!(!e.game.join("red4ext/plugins/Cool/Cool.dll").exists());
    }

    #[test]
    fn disabling_keeps_the_logs_but_drops_empty_folders() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[("red4ext/plugins/Cool/Cool.dll", "c"), ("r6/scripts/Cool/a.reds", "a")]);
        let rec = inst(&e.lib, &z, "Cool").unwrap();
        e.lib.enable(&rec.name, false).unwrap();
        fs::write(e.game.join("red4ext/plugins/Cool/cool.log"), "log").unwrap();
        e.lib.disable(&rec.name).unwrap();
        assert!(e.game.join("red4ext/plugins/Cool/cool.log").exists());
        assert!(!e.game.join("r6/scripts/Cool").exists());
    }

    #[cfg(windows)]
    #[test]
    fn a_locked_file_is_reported_and_everything_else_is_still_removed() {
        use std::os::windows::fs::OpenOptionsExt;
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[("bin/x64/winmm.dll", "w"), ("red4ext/RED4ext.dll", "r")]);
        let rec = inst(&e.lib, &z, "Loader").unwrap();
        e.lib.enable(&rec.name, false).unwrap();
        let held = fs::OpenOptions::new().read(true).share_mode(0).open(e.game.join("bin/x64/winmm.dll")).unwrap();
        let err = e.lib.uninstall(&rec.name).unwrap_err();
        assert!(matches!(err, AppError::FilesInUse(ref n) if n.contains("winmm.dll")));
        assert!(!e.game.join("red4ext/RED4ext.dll").exists());
        let left = e.lib.list().unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].installed_files, vec!["bin/x64/winmm.dll".to_string()]);
        drop(held);
        e.lib.uninstall(&rec.name).unwrap();
        assert!(!e.game.join("bin/x64/winmm.dll").exists());
        assert!(e.lib.list().unwrap().is_empty());
    }

    #[test]
    fn install_unzips_into_library_and_leaves_game_untouched() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        let rec = inst(&e.lib, &z, "m").unwrap();
        assert!(!rec.enabled);
        assert_eq!(rec.files, vec![CET.to_string()]);
        assert!(e.lib.root.join("m").join(CET).exists());
        assert_eq!(fs::read_dir(&e.game).unwrap().count(), 0);
        assert_eq!(e.lib.list().unwrap().len(), 1);
    }

    #[test]
    fn enable_copies_files_and_records_them() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        e.lib.enable("m", false).unwrap();
        assert_eq!(fs::read_to_string(e.game.join(CET)).unwrap(), "lua");
        let rec = &e.lib.list().unwrap()[0];
        assert!(rec.enabled);
        assert_eq!(rec.installed_files, vec![CET.to_string()]);
    }

    #[test]
    fn disable_removes_only_recorded_files() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        e.lib.enable("m", false).unwrap();
        let user = e.game.join("bin/x64/plugins/cyber_engine_tweaks/mods/m/user.json");
        fs::write(&user, "{}").unwrap();
        e.lib.disable("m").unwrap();
        assert!(user.exists());
        assert!(!e.game.join(CET).exists());
        assert!(!e.lib.list().unwrap()[0].enabled);
    }

    #[test]
    fn disable_keeps_unrelated_game_files_and_old_dirs() {
        let e = env();
        fs::create_dir_all(e.game.join("archive/pc/mod")).unwrap();
        fs::write(e.game.join("r6_other.txt"), "keep").unwrap();
        let z = make_zip(
            &e.work,
            "m.zip",
            &[("archive/pc/mod/a.archive", "a"), (CET, "lua")],
        );
        inst(&e.lib, &z, "m").unwrap();
        e.lib.enable("m", false).unwrap();
        e.lib.disable("m").unwrap();
        assert!(e.game.join("r6_other.txt").exists());
        assert!(e.game.join("archive/pc/mod").exists());
        assert!(!e.game.join("bin").exists());
    }

    #[test]
    fn uninstall_disables_then_deletes_library_copy() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        e.lib.enable("m", false).unwrap();
        e.lib.uninstall("m").unwrap();
        assert!(!e.game.join(CET).exists());
        assert!(!e.lib.root.join("m").exists());
        assert!(e.lib.list().unwrap().is_empty());
    }

    #[test]
    fn install_rejects_zip_slip_entry_and_writes_nothing() {
        let e = env();
        let z = make_zip(
            &e.work,
            "m.zip",
            &[("archive/pc/mod/ok.archive", "ok"), ("../evil.dll", "x")],
        );
        let err = inst(&e.lib, &z, "m").unwrap_err();
        assert!(matches!(err, AppError::UnsafePath(_)));
        assert!(!e.lib.root.join("m").exists());
        assert!(e.lib.list().unwrap().is_empty());
    }

    #[test]
    fn install_rejects_bad_names() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        for bad in ["", "..", "a/b", "a\\b", ".hidden"] {
            assert!(matches!(inst(&e.lib, &z, bad), Err(AppError::InvalidName(_))));
        }
    }

    #[test]
    fn taken_names_get_a_number() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        let first = inst(&e.lib, &z, "m").unwrap();
        let second = inst(&e.lib, &z, "m").unwrap();
        assert_eq!(first.name, "m");
        assert_eq!(second.name, "m (2)");
    }

    #[test]
    fn enable_reports_conflict_with_other_mod_and_force_overwrites() {
        let e = env();
        let za = make_zip(&e.work, "a.zip", &[("archive/pc/mod/x.archive", "from-a")]);
        let zb = make_zip(&e.work, "b.zip", &[("archive/pc/mod/x.archive", "from-b")]);
        inst(&e.lib, &za, "a").unwrap();
        inst(&e.lib, &zb, "b").unwrap();
        e.lib.enable("a", false).unwrap();
        match e.lib.enable("b", false) {
            Err(AppError::Conflict(list)) => assert_eq!(
                list,
                vec![Conflict {
                    file: "archive/pc/mod/x.archive".into(),
                    owner: Some("a".into())
                }]
            ),
            other => panic!("expected conflict, got {other:?}"),
        }
        e.lib.enable("b", true).unwrap();
        assert_eq!(
            fs::read_to_string(e.game.join("archive/pc/mod/x.archive")).unwrap(),
            "from-b"
        );
        e.lib.disable("b").unwrap();
        assert_eq!(
            fs::read_to_string(e.game.join("archive/pc/mod/x.archive")).unwrap(),
            "from-a"
        );
    }

    #[test]
    fn enable_reports_conflict_with_preexisting_game_file() {
        let e = env();
        fs::create_dir_all(e.game.join("archive/pc/mod")).unwrap();
        fs::write(e.game.join("archive/pc/mod/x.archive"), "old").unwrap();
        let z = make_zip(&e.work, "m.zip", &[("archive/pc/mod/x.archive", "new")]);
        inst(&e.lib, &z, "m").unwrap();
        match e.lib.enable("m", false) {
            Err(AppError::Conflict(list)) => assert_eq!(list[0].owner, None),
            other => panic!("expected conflict, got {other:?}"),
        }
        assert_eq!(
            fs::read_to_string(e.game.join("archive/pc/mod/x.archive")).unwrap(),
            "old"
        );
    }

    #[test]
    fn state_survives_reload() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        e.lib.enable("m", false).unwrap();
        let again = Library::new(e.lib.root.clone(), e.game.clone());
        assert_eq!(again.list().unwrap(), e.lib.list().unwrap());
    }

    #[test]
    fn adopt_copies_into_library_and_marks_enabled() {
        let e = env();
        let rel = "archive/pc/mod/hand.archive";
        fs::create_dir_all(e.game.join("archive/pc/mod")).unwrap();
        fs::write(e.game.join(rel), "data").unwrap();
        let rec = e.lib.adopt("hand", &[rel.to_string()]).unwrap();
        assert!(rec.enabled);
        assert!(e.lib.root.join("hand").join(rel).exists());
        e.lib.disable("hand").unwrap();
        assert!(!e.game.join(rel).exists());
        e.lib.enable("hand", false).unwrap();
        assert_eq!(fs::read_to_string(e.game.join(rel)).unwrap(), "data");
    }

    #[test]
    fn adopt_rejects_taken_names_and_missing_files() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        assert!(matches!(
            e.lib.adopt("m", &["x".to_string()]),
            Err(AppError::InvalidName(_))
        ));
        assert!(e.lib.adopt("ghost", &["nope.archive".to_string()]).is_err());
        assert!(!e.lib.root.join("ghost").exists());
    }

    #[test]
    fn enable_unknown_mod_is_not_found() {
        let e = env();
        assert!(matches!(
            e.lib.enable("nope", false),
            Err(AppError::NotFound(_))
        ));
    }

    fn rich_zip(e: &Env) -> PathBuf {
        make_zip(
            &e.work,
            "Cool-1.2.3.zip",
            &[
                (CET, "lua"),
                ("preview.png", "PNGDATA"),
                ("README.md", "# Cool\n\nDoes **things**."),
            ],
        )
    }

    #[test]
    fn install_reads_cover_description_and_version() {
        let e = env();
        let rec = inst(&e.lib, &rich_zip(&e), "Cool").unwrap();
        assert_eq!(rec.version, "1.2.3");
        assert!(rec.description.contains("Does things."));
        assert_eq!(rec.image.as_deref(), Some("cover.png"));
        assert!(e.lib.root.join("Cool/.meta/cover.png").exists());
        assert_eq!(rec.files, vec![CET.to_string()]);
        e.lib.enable("Cool", false).unwrap();
        assert!(!e.game.join("preview.png").exists());
        assert!(!e.game.join("README.md").exists());
    }

    #[test]
    fn metadata_survives_reload() {
        let e = env();
        let rec = inst(&e.lib, &rich_zip(&e), "Cool").unwrap();
        let again = Library::new(e.lib.root.clone(), e.game.clone());
        assert_eq!(again.list().unwrap()[0], rec);
    }

    #[test]
    fn old_state_without_new_fields_still_loads() {
        let e = env();
        fs::create_dir_all(&e.lib.root).unwrap();
        fs::write(
            e.lib.root.join("state.json"),
            r#"[{"name":"old","enabled":false,"files":[],"installed_files":[],"created_dirs":[]}]"#,
        )
        .unwrap();
        let list = e.lib.list().unwrap();
        assert_eq!(list[0].description, "");
        assert_eq!(list[0].image, None);
    }

    #[test]
    fn set_info_updates_text_and_image() {
        let e = env();
        inst(&e.lib, &rich_zip(&e), "Cool").unwrap();
        let pic = e.work.join("new.jpg");
        fs::write(&pic, "JPGDATA").unwrap();
        let rec = e.lib.set_info("Cool", "  new text ", " 9.9 ", Some(&pic)).unwrap();
        assert_eq!(rec.description, "new text");
        assert_eq!(rec.version, "9.9");
        assert_eq!(rec.image.as_deref(), Some("cover.jpg"));
        assert!(!e.lib.root.join("Cool/.meta/cover.png").exists());
        let url = e.lib.image_data_url("Cool").unwrap().unwrap();
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn set_info_rejects_non_images_and_unknown_mods() {
        let e = env();
        inst(&e.lib, &rich_zip(&e), "Cool").unwrap();
        let bad = e.work.join("x.exe");
        fs::write(&bad, "x").unwrap();
        assert!(e.lib.set_info("Cool", "", "", Some(&bad)).is_err());
        assert!(matches!(
            e.lib.set_info("ghost", "", "", None),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn image_data_url_is_none_without_a_cover() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        assert_eq!(e.lib.image_data_url("m").unwrap(), None);
    }

    #[test]
    fn exported_mod_installs_back_with_its_metadata() {
        let e = env();
        inst(&e.lib, &rich_zip(&e), "Cool").unwrap();
        e.lib.set_info("Cool", "my text", "9.9", None).unwrap();
        let out = e.work.join("out.zip");
        e.lib.export_mod("Cool", &out).unwrap();

        let other = env();
        let rec = inst(&other.lib, &out, "ignored").unwrap();
        assert_eq!(rec.name, "Cool");
        assert_eq!(rec.version, "9.9");
        assert_eq!(rec.description, "my text");
        assert_eq!(rec.image.as_deref(), Some("cover.png"));
        assert_eq!(rec.files, vec![CET.to_string()]);
    }

    #[test]
    fn exporting_reports_progress_up_to_the_file_count() {
        let e = env();
        let z = make_zip(&e.work, "a.zip", &[("archive/pc/mod/a.archive", "a"), ("archive/pc/mod/b.archive", "b")]);
        let r = inst(&e.lib, &z, "Cool").unwrap();
        let out = e.work.join("out.zip");
        let mut seen = Vec::new();
        e.lib.export_pack_with(&[r.name.clone()], &out, &mut |d, t| seen.push((d, t))).unwrap();
        assert_eq!(seen, vec![(0, 2), (1, 2), (2, 2)]);
    }

    #[test]
    fn exported_pack_installs_every_mod() {
        let e = env();
        let za = make_zip(&e.work, "a.zip", &[("archive/pc/mod/a.archive", "A")]);
        let zb = make_zip(&e.work, "b.zip", &[("archive/pc/mod/b.archive", "B")]);
        inst(&e.lib, &za, "a").unwrap();
        inst(&e.lib, &zb, "b").unwrap();
        let out = e.work.join("pack.zip");
        e.lib.export_pack(&["a".to_string(), "b".to_string()], &out).unwrap();

        let other = env();
        let added = other.lib.install_any(&out, "pack").unwrap();
        let names: Vec<&str> = added.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b"]);
        assert_eq!(
            fs::read_to_string(other.lib.root.join("b/archive/pc/mod/b.archive")).unwrap(),
            "B"
        );
        let (again, skipped) = other.lib.install_report(&out, "pack").unwrap();
        assert!(again.is_empty());
        assert_eq!(skipped, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(other.lib.list().unwrap().len(), 2);
    }

    #[test]
    fn a_pack_installs_only_the_mods_that_are_new_or_a_different_version() {
        let e = env();
        let za = make_zip(&e.work, "a.zip", &[("archive/pc/mod/a.archive", "A")]);
        let zb = make_zip(&e.work, "b.zip", &[("archive/pc/mod/b.archive", "B")]);
        inst(&e.lib, &za, "a").unwrap();
        inst(&e.lib, &zb, "b").unwrap();
        e.lib.set_info("b", "", "2.0", None).unwrap();
        let out = e.work.join("pack.zip");
        e.lib.export_pack(&["a".to_string(), "b".to_string()], &out).unwrap();

        let other = env();
        let zb_old = make_zip(&other.work, "bo.zip", &[("archive/pc/mod/b.archive", "B-old")]);
        inst(&other.lib, &zb_old, "b").unwrap();
        other.lib.set_info("b", "", "1.0", None).unwrap();
        let (added, skipped) = other.lib.install_report(&out, "pack").unwrap();
        let mut names: Vec<_> = added.iter().map(|r| r.name.as_str()).collect();
        names.sort();
        assert_eq!(names, vec!["a", "b (2)"]);
        assert!(skipped.is_empty());
        let (again, skipped) = other.lib.install_report(&out, "pack").unwrap();
        assert!(again.is_empty());
        assert_eq!(skipped.len(), 2);
    }

    #[test]
    fn a_name_with_a_counter_is_the_same_mod() {
        assert!(same_mod_name("Neon (2)", "neon"));
        assert!(same_mod_name("Neon", "NEON (12)"));
        assert!(!same_mod_name("Neon (v2)", "neon"));
        assert!(!same_mod_name("Neon 2", "neon"));
        assert!(!same_mod_name("()", ""));
    }

    #[test]
    fn pack_with_zip_slip_installs_nothing() {
        let e = env();
        let manifest = r#"{"format":1,"mods":[{"folder":"m1","name":"x"}]}"#;
        let z = make_zip(
            &e.work,
            "evil.zip",
            &[
                ("launcher-pack.json", manifest),
                ("mods/m1/archive/pc/mod/ok.archive", "ok"),
                ("mods/m1/../../evil.dll", "x"),
            ],
        );
        assert!(matches!(e.lib.install_any(&z, "p"), Err(AppError::UnsafePath(_))));
        assert!(!e.lib.root.join("x").exists());
        assert!(e.lib.list().unwrap().is_empty());
    }

    #[test]
    fn pack_rejects_unsafe_folder_names() {
        let e = env();
        let manifest = r#"{"format":1,"mods":[{"folder":"../x","name":"x"}]}"#;
        let z = make_zip(&e.work, "evil.zip", &[("launcher-pack.json", manifest)]);
        assert!(matches!(e.lib.install_any(&z, "p"), Err(AppError::UnsafePath(_))));
    }

    #[test]
    fn tag_record_sets_source_version_and_description() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        e.lib.tag_record("m", "nightcoop", "0.1.1-beta.2", "  The second beta ").unwrap();
        let rec = &e.lib.list().unwrap()[0];
        assert_eq!(rec.source.as_deref(), Some("nightcoop"));
        assert_eq!(rec.version, "0.1.1-beta.2");
        assert_eq!(rec.description, "The second beta");
        assert!(matches!(
            e.lib.tag_record("ghost", "x", "1", ""),
            Err(AppError::NotFound(_))
        ));
    }

    fn tagged_pair(e: &Env) {
        let a = make_zip(&e.work, "a.zip", &[("archive/pc/mod/nc.archive", "from-a")]);
        let b = make_zip(&e.work, "b.zip", &[("archive/pc/mod/nc.archive", "from-b")]);
        inst(&e.lib, &a, "NC 1").unwrap();
        inst(&e.lib, &b, "NC 2").unwrap();
        e.lib.tag_record("NC 1", "nightcoop", "1", "").unwrap();
        e.lib.tag_record("NC 2", "nightcoop", "2", "").unwrap();
    }

    #[test]
    fn activating_a_tagged_build_replaces_the_active_one() {
        let e = env();
        tagged_pair(&e);
        e.lib.activate_exclusive("NC 1", false).unwrap();
        e.lib.activate_exclusive("NC 2", false).unwrap();
        let list = e.lib.list().unwrap();
        assert!(!list[0].enabled);
        assert!(list[1].enabled);
        assert_eq!(
            fs::read_to_string(e.game.join("archive/pc/mod/nc.archive")).unwrap(),
            "from-b"
        );
    }

    #[test]
    fn activating_asks_before_replacing_untracked_files() {
        let e = env();
        tagged_pair(&e);
        fs::create_dir_all(e.game.join("archive/pc/mod")).unwrap();
        fs::write(e.game.join("archive/pc/mod/nc.archive"), "by-hand").unwrap();
        match e.lib.activate_exclusive("NC 1", false) {
            Err(AppError::Conflict(list)) => assert_eq!(list[0].owner, None),
            other => panic!("expected conflict, got {other:?}"),
        }
        assert!(!e.lib.list().unwrap()[0].enabled);
        assert_eq!(
            fs::read_to_string(e.game.join("archive/pc/mod/nc.archive")).unwrap(),
            "by-hand"
        );
        e.lib.activate_exclusive("NC 1", true).unwrap();
        assert!(e.lib.list().unwrap()[0].enabled);
    }

    #[test]
    fn untagged_mods_are_never_switched_off_silently() {
        let e = env();
        tagged_pair(&e);
        let other = make_zip(&e.work, "o.zip", &[("archive/pc/mod/nc.archive", "other")]);
        inst(&e.lib, &other, "Other").unwrap();
        e.lib.enable("Other", false).unwrap();
        match e.lib.activate_exclusive("NC 1", false) {
            Err(AppError::Conflict(list)) => assert_eq!(list[0].owner.as_deref(), Some("Other")),
            other => panic!("expected conflict, got {other:?}"),
        }
        assert!(e.lib.list().unwrap()[2].enabled);
    }

    #[test]
    fn a_nexus_file_name_gives_the_mod_number_and_version() {
        let e = env();
        let z = make_zip(&e.work, "Cool Mod-107-1-16-3-1760000000.zip", &[(CET, "lua")]);
        let rec = inst(&e.lib, &z, "Cool Mod").unwrap();
        assert_eq!(rec.nexus_id, Some(107));
        assert_eq!(rec.version, "1.16.3");
        let plain = make_zip(&e.work, "plain.zip", &[(CET, "lua")]);
        assert_eq!(inst(&e.lib, &plain, "plain").unwrap().nexus_id, None);
    }

    #[test]
    fn the_nexus_link_can_be_set_and_removed_by_hand() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        inst(&e.lib, &z, "m").unwrap();
        assert_eq!(e.lib.set_nexus_id("m", Some(55)).unwrap().nexus_id, Some(55));
        assert_eq!(e.lib.list().unwrap()[0].nexus_id, Some(55));
        assert_eq!(e.lib.set_nexus_id("m", None).unwrap().nexus_id, None);
        assert!(matches!(e.lib.set_nexus_id("ghost", Some(1)), Err(AppError::NotFound(_))));
    }

    fn put_game(e: &Env, rel: &str, body: &str) {
        let p = e.game.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    fn game_text(e: &Env, rel: &str) -> String {
        fs::read_to_string(e.game.join(rel)).unwrap()
    }

    const X: &str = "archive/pc/mod/x.archive";

    #[test]
    fn a_forced_enable_keeps_the_original_and_disable_puts_it_back() {
        let e = env();
        put_game(&e, X, "mine");
        let z = make_zip(&e.work, "m.zip", &[(X, "theirs")]);
        inst(&e.lib, &z, "m").unwrap();
        assert!(matches!(e.lib.enable("m", false), Err(AppError::Conflict(_))));
        e.lib.enable("m", true).unwrap();
        assert_eq!(game_text(&e, X), "theirs");
        assert_eq!(e.lib.list().unwrap()[0].backups.len(), 1);
        e.lib.disable("m").unwrap();
        assert_eq!(game_text(&e, X), "mine");
        assert!(e.lib.list().unwrap()[0].backups.is_empty());
        assert!(!e.lib.root.join(".overwritten/m").exists());
    }

    #[test]
    fn disabling_the_overwriting_mod_gives_the_other_mod_its_file_back() {
        let e = env();
        let za = make_zip(&e.work, "a.zip", &[(X, "from-a")]);
        let zb = make_zip(&e.work, "b.zip", &[(X, "from-b")]);
        inst(&e.lib, &za, "a").unwrap();
        inst(&e.lib, &zb, "b").unwrap();
        e.lib.enable("a", false).unwrap();
        e.lib.enable("b", true).unwrap();
        assert_eq!(game_text(&e, X), "from-b");
        assert!(e.lib.list().unwrap()[0].installed_files.is_empty());
        e.lib.disable("b").unwrap();
        assert_eq!(game_text(&e, X), "from-a");
        assert_eq!(e.lib.list().unwrap()[0].installed_files, vec![X.to_string()]);
        e.lib.disable("a").unwrap();
        assert!(!e.game.join(X).exists());
    }

    #[test]
    fn a_mod_that_was_switched_off_meanwhile_is_not_brought_back() {
        let e = env();
        let za = make_zip(&e.work, "a.zip", &[(X, "from-a")]);
        let zb = make_zip(&e.work, "b.zip", &[(X, "from-b")]);
        inst(&e.lib, &za, "a").unwrap();
        inst(&e.lib, &zb, "b").unwrap();
        e.lib.enable("a", false).unwrap();
        e.lib.enable("b", true).unwrap();
        e.lib.disable("a").unwrap();
        assert_eq!(game_text(&e, X), "from-b");
        e.lib.disable("b").unwrap();
        assert!(!e.game.join(X).exists());
    }

    #[test]
    fn a_failed_enable_leaves_every_original_file_untouched() {
        let e = env();
        put_game(&e, "archive/pc/mod/a.archive", "mine");
        let z = make_zip(
            &e.work,
            "m.zip",
            &[("archive/pc/mod/a.archive", "theirs"), ("bin/x64/plugins/new/b.dll", "dll")],
        );
        inst(&e.lib, &z, "m").unwrap();
        fs::remove_file(e.lib.root.join("m/bin/x64/plugins/new/b.dll")).unwrap();
        assert!(e.lib.enable("m", true).is_err());
        assert_eq!(game_text(&e, "archive/pc/mod/a.archive"), "mine");
        assert!(!e.game.join("bin").exists());
        assert!(!e.lib.list().unwrap()[0].enabled);
        assert!(!e.lib.root.join(".overwritten/m").exists());
    }

    #[test]
    fn changes_a_mod_made_to_its_own_files_survive_a_disable() {
        let e = env();
        let z = make_zip(&e.work, "m.zip", &[(CET, "original")]);
        inst(&e.lib, &z, "m").unwrap();
        e.lib.enable("m", false).unwrap();
        fs::write(e.game.join(CET), "edited by the mod, longer").unwrap();
        e.lib.disable("m").unwrap();
        assert!(!e.game.join(CET).exists());
        e.lib.enable("m", false).unwrap();
        assert_eq!(game_text(&e, CET), "edited by the mod, longer");
    }

    #[test]
    fn a_leftover_folder_with_no_record_is_cleared_before_installing() {
        let e = env();
        fs::create_dir_all(e.lib.root.join("m")).unwrap();
        fs::write(e.lib.root.join("m/junk.txt"), "old").unwrap();
        let z = make_zip(&e.work, "m.zip", &[(CET, "lua")]);
        let rec = inst(&e.lib, &z, "m").unwrap();
        assert_eq!(rec.name, "m");
        assert!(!e.lib.root.join("m/junk.txt").exists());
    }

    #[test]
    fn uninstall_saves_the_state_before_deleting_the_folder_and_clears_backups() {
        let e = env();
        put_game(&e, X, "mine");
        let z = make_zip(&e.work, "m.zip", &[(X, "theirs")]);
        inst(&e.lib, &z, "m").unwrap();
        e.lib.enable("m", true).unwrap();
        e.lib.uninstall("m").unwrap();
        assert_eq!(game_text(&e, X), "mine");
        assert!(e.lib.list().unwrap().is_empty());
        assert!(!e.lib.root.join("m").exists());
        assert!(!e.lib.root.join(".overwritten/m").exists());
    }

    #[test]
    fn a_failed_switch_between_builds_turns_the_old_one_back_on() {
        let e = env();
        let a = make_zip(&e.work, "a.zip", &[(X, "from-a")]);
        let b = make_zip(&e.work, "b.zip", &[(X, "from-b")]);
        inst(&e.lib, &a, "NC 1").unwrap();
        inst(&e.lib, &b, "NC 2").unwrap();
        e.lib.tag_record("NC 1", "nightcoop", "1", "").unwrap();
        e.lib.tag_record("NC 2", "nightcoop", "2", "").unwrap();
        e.lib.activate_exclusive("NC 1", false).unwrap();
        fs::remove_file(e.lib.root.join("NC 2").join(X)).unwrap();
        assert!(e.lib.activate_exclusive("NC 2", false).is_err());
        let list = e.lib.list().unwrap();
        assert!(list[0].enabled && !list[1].enabled);
        assert_eq!(game_text(&e, X), "from-a");
    }

    #[test]
    fn several_installs_at_once_all_survive_with_their_own_names() {
        let e = env();
        let zips: Vec<PathBuf> = (0..4)
            .map(|i| make_zip(&e.work, &format!("z{i}.zip"), &[(&format!("archive/pc/mod/f{i}.archive"), "x")]))
            .collect();
        let handles: Vec<_> = zips
            .into_iter()
            .map(|z| {
                let root = e.lib.root.clone();
                let game = e.game.clone();
                std::thread::spawn(move || Library::new(root, game).install_any(&z, "same").unwrap())
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let mut names: Vec<String> = e.lib.list().unwrap().into_iter().map(|r| r.name).collect();
        names.sort();
        assert_eq!(names, vec!["same", "same (2)", "same (3)", "same (4)"]);
        for n in &names {
            assert!(e.lib.root.join(n).is_dir());
        }
    }

    #[test]
    fn an_install_and_an_enable_at_the_same_time_do_not_lose_each_other() {
        let e = env();
        let z1 = make_zip(&e.work, "one.zip", &[(X, "one")]);
        inst(&e.lib, &z1, "one").unwrap();
        let z2 = make_zip(&e.work, "two.zip", &[("archive/pc/mod/y.archive", "two")]);
        let root = e.lib.root.clone();
        let game = e.game.clone();
        let installer = std::thread::spawn(move || Library::new(root, game).install_any(&z2, "two").unwrap());
        e.lib.enable("one", false).unwrap();
        installer.join().unwrap();
        let list = e.lib.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().find(|r| r.name == "one").unwrap().enabled);
    }

    #[test]
    fn archives_over_the_limits_are_refused_and_leave_nothing_behind() {
        let e = env();
        let z = make_zip(
            &e.work,
            "big.zip",
            &[
                ("archive/pc/mod/a.archive", "0123456789"),
                ("archive/pc/mod/b.archive", "0123456789"),
                ("archive/pc/mod/c.archive", "0123456789"),
            ],
        );
        let too_many = Limits { max_entries: 2, ..Limits::default() };
        assert!(matches!(e.lib.install_any_with(&z, "m", &too_many), Err(AppError::Zip(_))));
        let too_big = Limits { max_total: 25, ..Limits::default() };
        assert!(matches!(e.lib.install_any_with(&z, "m", &too_big), Err(AppError::Zip(_))));
        let file_too_big = Limits { max_file: 5, ..Limits::default() };
        assert!(matches!(e.lib.install_any_with(&z, "m", &file_too_big), Err(AppError::Zip(_))));
        assert!(!e.lib.root.join("m").exists());
        assert!(e.lib.list().unwrap().is_empty());
        assert!(e.lib.install_any_with(&z, "m", &Limits::default()).is_ok());
    }

    #[test]
    fn reserved_and_awkward_mod_names_are_refused_or_cleaned() {
        for bad in ["CON", "nul", "com1", "LPT9", "aux.txt", "trailing.", "trailing ", " leading", "state.json", "STATE.JSON", "a\u{7}b"] {
            assert!(matches!(validate_name(bad), Err(AppError::InvalidName(_))), "{bad:?}");
        }
        assert!(validate_name("Cool Mod (2)").is_ok());
        assert!(validate_name("console").is_ok());
        assert_eq!(sanitize_name("x. "), "x");
        assert_eq!(sanitize_name("con"), "_con");
        assert_eq!(sanitize_name(". .overwritten"), "overwritten");
        assert_eq!(sanitize_name(" . . .hidden"), "hidden");
        assert_eq!(sanitize_name("state.json.tmp"), "_state.json.tmp");
        assert_eq!(sanitize_name("a:b"), "a_b");
    }

    #[test]
    fn export_of_unknown_mod_fails_without_leaving_a_file() {
        let e = env();
        let out = e.work.join("nope.zip");
        assert!(e.lib.export_mod("nope", &out).is_err());
        assert!(!out.exists());
    }
}
