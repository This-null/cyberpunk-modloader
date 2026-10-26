use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

pub const MAX_IMAGE: u64 = 5 * 1024 * 1024;
pub const MAX_DESCRIPTION: usize = 1500;
pub const MAX_VERSION: usize = 40;
const KEYWORDS: [&str; 7] = ["preview", "cover", "icon", "thumb", "banner", "logo", "screenshot"];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModManifest {
    #[serde(default)]
    pub format: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEntry {
    pub folder: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    #[serde(default)]
    pub format: u32,
    pub mods: Vec<PackEntry>,
}

impl From<&PackEntry> for ModManifest {
    fn from(e: &PackEntry) -> Self {
        ModManifest {
            format: 1,
            name: e.name.clone(),
            version: e.version.clone(),
            description: e.description.clone(),
            cover: e.cover.clone(),
        }
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub fn image_ext(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        Some("png")
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some("jpg")
    } else if lower.ends_with(".webp") {
        Some("webp")
    } else {
        None
    }
}

pub fn is_image(name: &str) -> bool {
    image_ext(name).is_some()
}

pub fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

pub fn pick_cover(candidates: &[(String, u64)]) -> Option<usize> {
    let keyed = candidates
        .iter()
        .enumerate()
        .filter(|(_, (n, _))| {
            let lower = file_name(n).to_ascii_lowercase();
            KEYWORDS.iter().any(|k| lower.contains(k))
        })
        .max_by_key(|(_, (_, size))| *size)
        .map(|(i, _)| i);
    keyed.or_else(|| {
        candidates
            .iter()
            .enumerate()
            .max_by_key(|(_, (_, size))| *size)
            .map(|(i, _)| i)
    })
}

pub fn pick_readme(names: &[String]) -> Option<usize> {
    names
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            let lower = file_name(n).to_ascii_lowercase();
            (lower.starts_with("readme") || lower.starts_with("description"))
                && (lower.ends_with(".md") || lower.ends_with(".txt"))
        })
        .min_by_key(|(_, n)| (n.matches('/').count(), n.len()))
        .map(|(i, _)| i)
}

fn regex(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid pattern"))
}

pub fn clean_description(text: &str) -> String {
    static IMAGES: OnceLock<Regex> = OnceLock::new();
    static LINKS: OnceLock<Regex> = OnceLock::new();
    static TAGS: OnceLock<Regex> = OnceLock::new();
    static BLANKS: OnceLock<Regex> = OnceLock::new();
    let text = text.replace('\r', "");
    let text = regex(&IMAGES, r"!\[[^\]]*\]\([^)]*\)").replace_all(&text, "");
    let text = regex(&LINKS, r"\[([^\]]*)\]\([^)]*\)").replace_all(&text, "$1");
    let text = regex(&TAGS, r"<[^>]+>").replace_all(&text, "");
    let lines: Vec<String> = text
        .lines()
        .map(|l| {
            l.trim_start_matches('#')
                .replace("**", "")
                .replace("__", "")
                .replace('`', "")
                .trim()
                .to_string()
        })
        .collect();
    let joined = lines.join("\n");
    let collapsed = regex(&BLANKS, r"\n{3,}").replace_all(&joined, "\n\n");
    let trimmed = collapsed.trim();
    if trimmed.chars().count() <= MAX_DESCRIPTION {
        return trimmed.to_string();
    }
    let cut: String = trimmed.chars().take(MAX_DESCRIPTION).collect();
    format!("{}…", cut.trim_end())
}

pub fn version_from_name(stem: &str) -> String {
    static VERSION: OnceLock<Regex> = OnceLock::new();
    regex(&VERSION, r"(\d+(?:\.\d+){1,3})")
        .find(stem)
        .map(|m| m.as_str().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cands(list: &[(&str, u64)]) -> Vec<(String, u64)> {
        list.iter().map(|(n, s)| (n.to_string(), *s)).collect()
    }

    #[test]
    fn cover_prefers_keyword_names() {
        let c = cands(&[("big.png", 900), ("docs/Preview.jpg", 10), ("cover.png", 50)]);
        assert_eq!(pick_cover(&c), Some(2));
    }

    #[test]
    fn cover_falls_back_to_largest() {
        let c = cands(&[("a.png", 10), ("b.jpg", 90), ("c.webp", 40)]);
        assert_eq!(pick_cover(&c), Some(1));
    }

    #[test]
    fn cover_none_without_candidates() {
        assert_eq!(pick_cover(&[]), None);
    }

    #[test]
    fn image_detection_covers_common_formats() {
        assert!(is_image("A.PNG") && is_image("b.jpeg") && is_image("c.webp"));
        assert!(!is_image("d.gif") && !is_image("e.archive"));
        assert_eq!(image_ext("x.JPEG"), Some("jpg"));
    }

    #[test]
    fn readme_prefers_shallow_paths() {
        let n: Vec<String> = ["docs/readme.md", "README.txt", "notes.txt"].map(String::from).to_vec();
        assert_eq!(pick_readme(&n), Some(1));
        assert_eq!(pick_readme(&["x.md".to_string()]), None);
    }

    #[test]
    fn description_strips_markdown() {
        let md = "# Cool Mod\n\n**Adds** a [thing](http://x.y) with `code`.\n![shot](a.png)\n<b>bold</b>\n\n\n\nEnd";
        assert_eq!(clean_description(md), "Cool Mod\n\nAdds a thing with code.\n\nbold\n\nEnd");
    }

    #[test]
    fn description_is_truncated_on_char_boundaries() {
        let long = "é".repeat(MAX_DESCRIPTION + 50);
        let out = clean_description(&long);
        assert_eq!(out.chars().count(), MAX_DESCRIPTION + 1);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn version_comes_from_the_file_name() {
        assert_eq!(version_from_name("Cool Mod-1.2.3"), "1.2.3");
        assert_eq!(version_from_name("mod_v2.0"), "2.0");
        assert_eq!(version_from_name("Mod 2077"), "");
        assert_eq!(version_from_name("plain"), "");
    }
}
