use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{ProcessesToUpdate, System};

const GAME_NAME: &str = "Cyberpunk 2077";
const GAME_EXE: &str = "Cyberpunk2077.exe";

pub fn parse_library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("\"path\"")?;
            let body = &rest[rest.find('"')? + 1..];
            let value = &body[..body.rfind('"')?];
            Some(PathBuf::from(value.replace("\\\\", "\\")))
        })
        .collect()
}

#[derive(Deserialize)]
struct EpicManifest {
    #[serde(rename = "DisplayName")]
    display_name: String,
    #[serde(rename = "InstallLocation")]
    install_location: String,
}

pub fn parse_epic_manifest(json: &str) -> Option<PathBuf> {
    let m: EpicManifest = serde_json::from_str(json).ok()?;
    (m.display_name == GAME_NAME).then(|| PathBuf::from(m.install_location))
}

pub fn is_valid_game_dir(p: &Path) -> bool {
    p.join("bin").join("x64").join(GAME_EXE).is_file()
}

pub fn valid_dirs(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for c in candidates {
        if !is_valid_game_dir(&c) {
            continue;
        }
        let key = c.to_string_lossy().to_ascii_lowercase().replace('\\', "/");
        let seen = out
            .iter()
            .any(|o| o.to_string_lossy().to_ascii_lowercase().replace('\\', "/") == key);
        if !seen {
            out.push(c);
        }
    }
    out
}

#[cfg(windows)]
fn steam_path() -> Option<PathBuf> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\Valve\\Steam")
        .ok()?;
    let value: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(value))
}

#[cfg(not(windows))]
fn steam_path() -> Option<PathBuf> {
    None
}

fn steam_candidates() -> Vec<PathBuf> {
    let Some(steam) = steam_path() else {
        return Vec::new();
    };
    let mut libraries = vec![steam.clone()];
    if let Ok(vdf) = fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf")) {
        libraries.extend(parse_library_paths(&vdf));
    }
    libraries
        .into_iter()
        .map(|l| l.join("steamapps").join("common").join(GAME_NAME))
        .collect()
}

fn epic_candidates() -> Vec<PathBuf> {
    let Some(data) = std::env::var_os("ProgramData") else {
        return Vec::new();
    };
    let dir = PathBuf::from(data)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests");
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "item"))
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .filter_map(|text| parse_epic_manifest(&text))
        .collect()
}

pub fn find_game_dirs() -> Vec<PathBuf> {
    let mut all = steam_candidates();
    all.extend(epic_candidates());
    valid_dirs(all)
}

pub fn is_game_running() -> bool {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.processes()
        .values()
        .any(|p| p.name().to_string_lossy().eq_ignore_ascii_case(GAME_EXE))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VDF: &str = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}"#;

    fn fake_game(root: &Path) -> PathBuf {
        let dir = root.join("game");
        fs::create_dir_all(dir.join("bin").join("x64")).unwrap();
        fs::write(dir.join("bin").join("x64").join(GAME_EXE), "").unwrap();
        dir
    }

    #[test]
    fn parses_every_steam_library() {
        assert_eq!(
            parse_library_paths(VDF),
            vec![
                PathBuf::from("C:\\Program Files (x86)\\Steam"),
                PathBuf::from("D:\\SteamLibrary")
            ]
        );
    }

    #[test]
    fn epic_manifest_matches_the_game() {
        let json = r#"{"DisplayName":"Cyberpunk 2077","InstallLocation":"D:\\Epic Games\\Cyberpunk2077"}"#;
        assert_eq!(
            parse_epic_manifest(json),
            Some(PathBuf::from("D:\\Epic Games\\Cyberpunk2077"))
        );
    }

    #[test]
    fn epic_manifest_ignores_other_games() {
        let json = r#"{"DisplayName":"Red Dead Redemption 2","InstallLocation":"D:\\Epic Games\\RDR2"}"#;
        assert_eq!(parse_epic_manifest(json), None);
    }

    #[test]
    fn epic_manifest_ignores_garbage() {
        assert_eq!(parse_epic_manifest("not json"), None);
        assert_eq!(parse_epic_manifest("{}"), None);
    }

    #[test]
    fn valid_dir_needs_the_exe() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_valid_game_dir(tmp.path()));
        let game = fake_game(tmp.path());
        assert!(is_valid_game_dir(&game));
    }

    #[test]
    fn valid_dirs_filters_and_dedupes() {
        let tmp = tempfile::tempdir().unwrap();
        let game = fake_game(tmp.path());
        let missing = tmp.path().join("missing");
        let found = valid_dirs(vec![game.clone(), missing, game.clone()]);
        assert_eq!(found, vec![game]);
    }
}
