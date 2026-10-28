use serde::Serialize;
use std::fs;
use std::path::Path;

const MARKERS: [(&str, &str); 8] = [
    ("cet", "bin/x64/plugins/cyber_engine_tweaks.asi"),
    ("red4ext", "red4ext/RED4ext.dll"),
    ("archivexl", "red4ext/plugins/ArchiveXL/ArchiveXL.dll"),
    ("tweakxl", "red4ext/plugins/TweakXL/TweakXL.dll"),
    ("redscript", "engine/tools/scc.exe"),
    ("codeware", "red4ext/plugins/Codeware/Codeware.dll"),
    ("audioware", "red4ext/plugins/audioware/audioware.dll"),
    ("inputloader", "red4ext/plugins/input_loader/input_loader.dll"),
];

const REMOVABLE: [(&str, &[&str]); 8] = [
    ("cet", &["bin/x64/plugins/cyber_engine_tweaks.asi", "bin/x64/plugins/cyber_engine_tweaks/", "bin/x64/version.dll", "bin/x64/global.ini"]),
    ("red4ext", &["red4ext/", "bin/x64/winmm.dll"]),
    ("archivexl", &["red4ext/plugins/ArchiveXL/"]),
    ("tweakxl", &["red4ext/plugins/TweakXL/"]),
    ("redscript", &["engine/tools/scc.exe"]),
    ("codeware", &["red4ext/plugins/Codeware/"]),
    ("audioware", &["red4ext/plugins/audioware/"]),
    ("inputloader", &["red4ext/plugins/input_loader/"]),
];

pub fn removal_targets(game: &Path, id: &str) -> Vec<std::path::PathBuf> {
    let Some((_, rels)) = REMOVABLE.iter().find(|(i, _)| *i == id) else { return Vec::new() };
    rels.iter()
        .filter_map(|rel| {
            let dir = rel.ends_with('/');
            let mut here = game.to_path_buf();
            for part in rel.trim_end_matches('/').split('/') {
                let found = fs::read_dir(&here).ok()?.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
                here = found.path();
            }
            (if dir { here.is_dir() } else { here.is_file() }).then_some(here)
        })
        .collect()
}

pub fn removal_plan(game: &Path, id: &str) -> Vec<String> {
    removal_targets(game, id)
        .iter()
        .map(|p| {
            let rel = p.strip_prefix(game).unwrap_or(p).to_string_lossy().replace('\\', "/");
            if p.is_dir() { format!("{rel}/") } else { rel }
        })
        .collect()
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FrameworkState {
    pub id: &'static str,
    pub present: bool,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct FrameworkReport {
    pub frameworks: Vec<FrameworkState>,
    pub old_loader: bool,
}

#[cfg(windows)]
pub fn file_version(path: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), 0, size, buf.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let mut ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let mut len = 0u32;
        let root: Vec<u16> = "\\\0".encode_utf16().collect();
        if VerQueryValueW(buf.as_ptr().cast(), root.as_ptr(), &mut ptr, &mut len) == 0 || ptr.is_null() || (len as usize) < std::mem::size_of::<VS_FIXEDFILEINFO>() {
            return None;
        }
        let info = &*(ptr as *const VS_FIXEDFILEINFO);
        let (a, b, c) = (info.dwFileVersionMS >> 16, info.dwFileVersionMS & 0xffff, info.dwFileVersionLS >> 16);
        (a | b | c != 0).then(|| format!("{a}.{b}.{c}"))
    }
}

#[cfg(not(windows))]
pub fn file_version(_: &Path) -> Option<String> {
    None
}

fn contains_bytes(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

pub fn old_red4ext_loader(game: &Path) -> bool {
    let Some(path) = resolve_ci(game, "bin/x64/d3d11.dll") else { return false };
    let Ok(meta) = fs::metadata(&path) else { return false };
    if meta.len() > 8 * 1024 * 1024 {
        return false;
    }
    let Ok(bytes) = fs::read(&path) else { return false };
    let wide: Vec<u8> = "RED4ext".encode_utf16().flat_map(u16::to_le_bytes).collect();
    contains_bytes(&bytes, b"RED4ext") || contains_bytes(&bytes, &wide)
}

fn resolve_ci(root: &Path, rel: &str) -> Option<std::path::PathBuf> {
    let mut here = root.to_path_buf();
    for part in rel.split('/') {
        let entries = fs::read_dir(&here).ok()?;
        let found = entries.flatten().find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        here = found.path();
    }
    here.is_file().then_some(here)
}

pub fn check(game: &Path) -> FrameworkReport {
    let frameworks = MARKERS
        .iter()
        .map(|(id, rel)| {
            let file = resolve_ci(game, rel);
            FrameworkState { id, present: file.is_some(), version: file.as_deref().and_then(file_version) }
        })
        .collect();
    FrameworkReport { frameworks, old_loader: old_red4ext_loader(game) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, rel: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b"x").unwrap();
    }

    #[test]
    fn present_frameworks_are_found_whatever_the_letter_case() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "bin/x64/plugins/cyber_engine_tweaks.asi");
        touch(dir.path(), "red4ext/plugins/archivexl/ARCHIVEXL.DLL");
        touch(dir.path(), "engine/tools/scc.exe");
        let state = check(dir.path()).frameworks;
        let on = |id: &str| state.iter().find(|s| s.id == id).unwrap().present;
        assert!(on("cet") && on("archivexl") && on("redscript"));
        assert!(!on("red4ext") && !on("tweakxl") && !on("codeware"));
    }

    #[test]
    fn an_old_red4ext_loader_is_told_apart_from_other_d3d11_wrappers() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("bin/x64")).unwrap();
        fs::write(dir.path().join("bin/x64/d3d11.dll"), b"MZ....ReShade....").unwrap();
        assert!(!old_red4ext_loader(dir.path()));
        fs::write(dir.path().join("bin/x64/d3d11.dll"), b"MZ....RED4ext loader....").unwrap();
        assert!(old_red4ext_loader(dir.path()));
        let wide: Vec<u8> = "RED4ext".encode_utf16().flat_map(u16::to_le_bytes).collect();
        fs::write(dir.path().join("bin/x64/d3d11.dll"), [b"MZ".as_slice(), &wide].concat()).unwrap();
        assert!(old_red4ext_loader(dir.path()));
    }

    #[cfg(windows)]
    #[test]
    fn windows_file_versions_are_read() {
        let v = file_version(Path::new("C:/Windows/System32/kernel32.dll")).unwrap();
        assert!(v.split('.').count() == 3 && v.split('.').all(|p| p.parse::<u32>().is_ok()), "{v}");
        assert!(file_version(Path::new("C:/no/such/file.dll")).is_none());
    }

    #[test]
    fn removal_plans_name_exactly_what_will_be_deleted() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "bin/x64/plugins/cyber_engine_tweaks.asi");
        touch(dir.path(), "bin/x64/plugins/cyber_engine_tweaks/mods/mine/init.lua");
        touch(dir.path(), "red4ext/plugins/ArchiveXL/ArchiveXL.dll");
        touch(dir.path(), "red4ext/plugins/Other/Other.dll");
        let cet = removal_plan(dir.path(), "cet");
        assert_eq!(cet, vec!["bin/x64/plugins/cyber_engine_tweaks.asi".to_string(), "bin/x64/plugins/cyber_engine_tweaks/".to_string()]);
        touch(dir.path(), "red4ext/RED4ext.dll");
        touch(dir.path(), "bin/x64/winmm.dll");
        assert_eq!(removal_plan(dir.path(), "red4ext"), vec!["red4ext/".to_string(), "bin/x64/winmm.dll".to_string()]);
        let axl = removal_targets(dir.path(), "archivexl");
        assert_eq!(axl.len(), 1);
        assert!(axl[0].is_dir() && axl[0].ends_with("ArchiveXL"));
        assert!(removal_targets(dir.path(), "tweakxl").is_empty() && removal_targets(dir.path(), "nope").is_empty());
    }

    #[test]
    fn a_missing_game_folder_reports_everything_absent() {
        let report = check(Path::new("Z:/no/such/dir"));
        assert!(report.frameworks.iter().all(|s| !s.present) && !report.old_loader);
    }
}
