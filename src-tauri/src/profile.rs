use crate::error::AppError;
use crate::library::ModRecord;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

pub const PREFIX: &str = "CP2077:";
const MAX_MODS: usize = 200;
const MAX_PACKED: usize = 64 * 1024;
const MAX_JSON: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileMod {
    pub source: String,
    #[serde(default)]
    pub mod_id: u64,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub format: u32,
    #[serde(default)]
    pub name: String,
    pub mods: Vec<ProfileMod>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Built {
    pub profile: Profile,
    pub skipped: Vec<String>,
}

fn clean(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string()
}

pub fn build(records: &[ModRecord], names: &[String], title: &str) -> Built {
    let mut mods: Vec<ProfileMod> = Vec::new();
    let mut skipped = Vec::new();
    for r in records.iter().filter(|r| names.iter().any(|n| n == &r.name)) {
        let entry = match (r.source.as_deref(), r.nexus_id) {
            (Some("nightcoop"), _) => Some(ProfileMod {
                source: "nightcoop".into(),
                mod_id: 0,
                version: clean(&r.version, 64),
                name: clean(&r.name, 120),
            }),
            (_, Some(id)) if id > 0 => Some(ProfileMod {
                source: "nexus".into(),
                mod_id: id,
                version: clean(&r.version, 64),
                name: clean(&r.name, 120),
            }),
            _ => None,
        };
        match entry {
            Some(e) => {
                let same = |m: &ProfileMod| m.source == e.source && m.mod_id == e.mod_id && m.mod_id != 0;
                match mods.iter().position(same) {
                    Some(i) if r.enabled => mods[i] = e,
                    Some(_) => {}
                    None => mods.push(e),
                }
            }
            None => skipped.push(r.name.clone()),
        }
    }
    mods.truncate(MAX_MODS);
    Built {
        profile: Profile { format: 1, name: clean(title, 80), mods },
        skipped,
    }
}

pub fn validate(p: &Profile) -> Result<(), AppError> {
    let bad = |what: &str| AppError::Unmappable(format!("invalid profile: {what}"));
    if p.format != 1 {
        return Err(bad("format"));
    }
    if p.mods.is_empty() || p.mods.len() > MAX_MODS {
        return Err(bad("size"));
    }
    for m in &p.mods {
        let version_ok = m.version.chars().count() <= 64 && m.version.chars().all(|c| c.is_ascii_alphanumeric() || " .-+_()".contains(c));
        let ok = match m.source.as_str() {
            "nexus" => m.mod_id > 0 && m.mod_id < 10_000_000_000,
            "nightcoop" => !m.version.is_empty(),
            _ => false,
        };
        if !ok || !version_ok || m.name.chars().count() > 120 || m.name.chars().any(|c| c.is_control()) {
            return Err(bad("mod"));
        }
    }
    Ok(())
}

pub fn encode(p: &Profile) -> Result<String, AppError> {
    let json = serde_json::to_vec(p).map_err(|e| AppError::State(e.to_string()))?;
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(&json)?;
    let packed = enc.finish()?;
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(packed)))
}

pub fn decode(text: &str) -> Result<Profile, AppError> {
    let bad = || AppError::Unmappable("invalid profile".into());
    let trimmed = text.trim();
    if trimmed.len() > MAX_JSON as usize * 2 {
        return Err(bad());
    }
    let json: Vec<u8> = if trimmed.starts_with('{') {
        trimmed.as_bytes().to_vec()
    } else {
        let body = trimmed.strip_prefix(PREFIX).ok_or_else(bad)?;
        if body.len() > MAX_PACKED * 2 {
            return Err(bad());
        }
        let packed = URL_SAFE_NO_PAD.decode(body.trim()).map_err(|_| bad())?;
        if packed.len() > MAX_PACKED {
            return Err(bad());
        }
        let mut out = Vec::new();
        DeflateDecoder::new(packed.as_slice())
            .take(MAX_JSON + 1)
            .read_to_end(&mut out)
            .map_err(|_| bad())?;
        if out.len() as u64 > MAX_JSON {
            return Err(bad());
        }
        out
    };
    let profile: Profile = serde_json::from_slice(&json).map_err(|_| bad())?;
    validate(&profile)?;
    Ok(profile)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PlanItem {
    pub source: String,
    pub mod_id: u64,
    pub version: String,
    pub name: String,
    pub installed: bool,
}

pub fn plan(p: &Profile, records: &[ModRecord]) -> Vec<PlanItem> {
    p.mods
        .iter()
        .map(|m| {
            let installed = records.iter().any(|r| match m.source.as_str() {
                "nexus" => r.nexus_id == Some(m.mod_id),
                "nightcoop" => r.source.as_deref() == Some("nightcoop") && r.version == m.version,
                _ => false,
            });
            PlanItem {
                source: m.source.clone(),
                mod_id: m.mod_id,
                version: m.version.clone(),
                name: if m.name.is_empty() { format!("#{}", m.mod_id) } else { m.name.clone() },
                installed,
            }
        })
        .collect()
}

pub fn is_short_code(s: &str) -> bool {
    let t = s.trim();
    t.len() == 8 && t.bytes().all(|b| matches!(b, b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'2'..=b'9'))
}

pub const SERVER: &str = "https://vlr.skydev.com.tr/api/cp2077/profiles";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Shared {
    pub code: String,
    pub expires: u64,
}

fn http(base: &str) -> Result<reqwest::blocking::Client, AppError> {
    reqwest::blocking::Client::builder()
        .user_agent(format!("{}/{}", crate::nexus::APP_NAME, env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(12))
        .https_only(base.starts_with("https://"))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Network(e.to_string()))
}

fn read_capped(resp: reqwest::blocking::Response, cap: u64) -> Result<Vec<u8>, AppError> {
    let mut buf = Vec::new();
    resp.take(cap + 1).read_to_end(&mut buf).map_err(|e| AppError::Network(e.to_string()))?;
    if buf.len() as u64 > cap {
        return Err(AppError::Network("the answer is too large".into()));
    }
    Ok(buf)
}

const MAX_BITS: u32 = 24;

pub fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut bits = 0;
    for b in bytes {
        if *b == 0 {
            bits += 8;
        } else {
            bits += b.leading_zeros();
            break;
        }
    }
    bits
}

pub fn solve(challenge: &str, data_hash: &str, bits: u32) -> u64 {
    let mut counter = 0u64;
    loop {
        let digest = Sha256::digest(format!("{challenge}:{data_hash}:{counter}").as_bytes());
        if leading_zero_bits(&digest) >= bits {
            return counter;
        }
        counter += 1;
    }
}

pub fn share(base: &str, encoded: &str, install_id: &str) -> Result<Shared, AppError> {
    decode(encoded)?;
    let client = http(base)?;
    let first = client
        .get(format!("{base}/challenge"))
        .send()
        .map_err(|e| AppError::Network(e.to_string()))?;
    if !first.status().is_success() {
        return Err(AppError::Network(format!("the profile server answered {}", first.status())));
    }
    let v: serde_json::Value = serde_json::from_slice(&read_capped(first, 4096)?).map_err(|e| AppError::Network(e.to_string()))?;
    let challenge = v.get("challenge").and_then(|c| c.as_str()).filter(|c| c.len() <= 100).ok_or_else(|| AppError::Network("bad answer".into()))?;
    let bits = v.get("bits").and_then(|b| b.as_u64()).filter(|b| *b <= MAX_BITS as u64).ok_or_else(|| AppError::Network("bad answer".into()))? as u32;
    let data_hash: String = Sha256::digest(encoded.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    let counter = solve(challenge, &data_hash, bits);
    let resp = client
        .post(base)
        .header("Content-Type", "application/json")
        .body(serde_json::json!({ "data": encoded, "id": install_id, "challenge": challenge, "counter": counter }).to_string())
        .send()
        .map_err(|e| AppError::Network(e.to_string()))?;
    if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(AppError::RateLimited);
    }
    if !resp.status().is_success() {
        return Err(AppError::Network(format!("the profile server answered {}", resp.status())));
    }
    let body = read_capped(resp, 4096)?;
    let v: serde_json::Value = serde_json::from_slice(&body).map_err(|e| AppError::Network(e.to_string()))?;
    let code = v.get("code").and_then(|c| c.as_str()).filter(|c| is_short_code(c)).ok_or_else(|| AppError::Network("bad answer".into()))?;
    Ok(Shared { code: code.to_string(), expires: v.get("expires").and_then(|e| e.as_u64()).unwrap_or(0) })
}

pub fn fetch(base: &str, code: &str) -> Result<String, AppError> {
    let code = code.trim();
    if !is_short_code(code) {
        return Err(AppError::BadLink("profile code".into()));
    }
    let resp = http(base)?
        .get(format!("{base}/{code}"))
        .send()
        .map_err(|e| AppError::Network(e.to_string()))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::NotFound("profile".into()));
    }
    if !resp.status().is_success() {
        return Err(AppError::Network(format!("the profile server answered {}", resp.status())));
    }
    let body = read_capped(resp, (MAX_PACKED * 2) as u64)?;
    let text = String::from_utf8(body).map_err(|e| AppError::Network(e.to_string()))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| AppError::Network(e.to_string()))?;
    let data = v.get("data").and_then(|d| d.as_str()).ok_or_else(|| AppError::Network("bad answer".into()))?;
    decode(data)?;
    Ok(data.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    fn rec(name: &str, source: Option<&str>, nexus: Option<u64>, version: &str, enabled: bool) -> ModRecord {
        ModRecord {
            name: name.into(),
            source: source.map(str::to_string),
            nexus_id: nexus,
            version: version.into(),
            enabled,
            ..ModRecord::default()
        }
    }

    fn sample() -> Vec<ModRecord> {
        vec![
            rec("NightCoop 0.1.1-beta.2", Some("nightcoop"), None, "0.1.1-beta.2", true),
            rec("CET", None, Some(107), "1.38.1", true),
            rec("CET old", None, Some(107), "1.37", false),
            rec("Hand made", None, None, "1", true),
            rec("redscript", None, Some(1511), "0.5.32", false),
        ]
    }

    fn names(r: &[ModRecord]) -> Vec<String> {
        r.iter().map(|m| m.name.clone()).collect()
    }

    #[test]
    fn a_profile_lists_sources_not_files_and_reports_what_it_could_not_include() {
        let r = sample();
        let b = build(&r, &names(&r), "My setup");
        assert_eq!(b.profile.name, "My setup");
        let ids: Vec<(&str, u64, &str)> = b.profile.mods.iter().map(|m| (m.source.as_str(), m.mod_id, m.version.as_str())).collect();
        assert_eq!(ids, vec![("nightcoop", 0, "0.1.1-beta.2"), ("nexus", 107, "1.38.1"), ("nexus", 1511, "0.5.32")]);
        assert_eq!(b.skipped, vec!["Hand made".to_string()]);
        let only = build(&r, &["redscript".to_string()], "x");
        assert_eq!(only.profile.mods.len(), 1);
    }

    #[test]
    fn a_code_round_trips_and_stays_short() {
        let mut r = Vec::new();
        for i in 1..=40u64 {
            r.push(rec(&format!("Some Mod Number {i}"), None, Some(1000 + i), "1.2.3", true));
        }
        let b = build(&r, &names(&r), "Big pack");
        let code = encode(&b.profile).unwrap();
        assert!(code.starts_with(PREFIX));
        assert!(code.len() < 1000, "{}", code.len());
        assert_eq!(decode(&code).unwrap(), b.profile);
        assert_eq!(decode(&format!("  {code}\n")).unwrap(), b.profile);
        let json = serde_json::to_string(&b.profile).unwrap();
        assert_eq!(decode(&json).unwrap(), b.profile);
    }

    #[test]
    fn broken_or_hostile_profiles_are_refused() {
        for bad in [
            "",
            "hello",
            "CP2077:",
            "CP2077:!!!!",
            "CP2077:AAAA",
            r#"{"format":2,"mods":[{"source":"nexus","mod_id":1}]}"#,
            r#"{"format":1,"mods":[]}"#,
            r#"{"format":1,"mods":[{"source":"evil","mod_id":1}]}"#,
            r#"{"format":1,"mods":[{"source":"nexus","mod_id":0}]}"#,
            r#"{"format":1,"mods":[{"source":"nightcoop","version":""}]}"#,
            r#"{"format":1,"mods":[{"source":"nexus","mod_id":1,"version":"1;rm -rf"}]}"#,
            r#"{"format":1,"mods":[{"source":"nexus","mod_id":1,"name":"a\u0000b"}]}"#,
        ] {
            assert!(decode(bad).is_err(), "{bad}");
        }
        let many: Vec<ProfileMod> = (1..=201)
            .map(|i| ProfileMod { source: "nexus".into(), mod_id: i, version: String::new(), name: String::new() })
            .collect();
        assert!(validate(&Profile { format: 1, name: String::new(), mods: many }).is_err());
        let bomb = URL_SAFE_NO_PAD.encode({
            let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
            e.write_all(&vec![b' '; 5_000_000]).unwrap();
            e.finish().unwrap()
        });
        assert!(decode(&format!("{PREFIX}{bomb}")).is_err());
    }

    #[test]
    fn the_plan_marks_what_is_already_installed_whatever_its_version() {
        let r = sample();
        let b = build(
            &[rec("A", None, Some(107), "9.9", true), rec("B", None, Some(5), "1", true), rec("NC", Some("nightcoop"), None, "0.1.2-beta", true)],
            &["A".into(), "B".into(), "NC".into()],
            "p",
        );
        let plan = plan(&b.profile, &r);
        let flags: Vec<(String, bool)> = plan.iter().map(|p| (p.name.clone(), p.installed)).collect();
        assert_eq!(flags, vec![("A".into(), true), ("B".into(), false), ("NC".into(), false)]);
    }

    #[test]
    fn short_codes_use_the_server_alphabet_only() {
        assert!(is_short_code("ABCD2345"));
        assert!(is_short_code(" ABCD2345 "));
        for bad in ["abcd2345", "ABCD234", "ABCD23456", "ABCDI234", "ABCDO234", "ABCD0123", "ABCD 345"] {
            assert!(!is_short_code(bad), "{bad}");
        }
    }

    fn tiny_server(answers: Vec<(&'static str, String)>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for (status, body) in answers {
                let (mut s, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(s.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let mut len = 0usize;
                loop {
                    let mut h = String::new();
                    reader.read_line(&mut h).unwrap();
                    if h.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body_in = vec![0u8; len];
                reader.read_exact(&mut body_in).unwrap();
                log.lock().unwrap().push(format!("{} {}", line.trim(), String::from_utf8_lossy(&body_in)));
                let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{body}", body.len());
            }
        });
        (format!("http://127.0.0.1:{port}"), seen)
    }

    #[test]
    fn the_work_found_has_enough_leading_zero_bits_and_is_bound_to_the_data() {
        let hash = "ab".repeat(32);
        let counter = solve("nonce.mac", &hash, 12);
        let digest = Sha256::digest(format!("nonce.mac:{hash}:{counter}").as_bytes());
        assert!(leading_zero_bits(&digest) >= 12);
        assert_eq!(leading_zero_bits(&[0, 0, 0x10]), 19);
        assert_eq!(leading_zero_bits(&[0x80]), 0);
        assert_eq!(leading_zero_bits(&[0x01]), 7);
    }

    #[test]
    fn sharing_does_the_work_posts_the_profile_and_fetching_reads_it_back() {
        let r = sample();
        let code = encode(&build(&r, &names(&r), "p").profile).unwrap();
        let challenge = r#"{"challenge":"abc.def","bits":10}"#.to_string();
        let reply = r#"{"code":"ABCD2345","expires":1790000000}"#.to_string();
        let fetched = format!(r#"{{"data":"{code}"}}"#);
        let (base, seen) = tiny_server(vec![
            ("200 OK", challenge),
            ("200 OK", reply),
            ("200 OK", fetched),
            ("404 Not Found", "{}".into()),
            ("200 OK", r#"{"data":"junk"}"#.into()),
            ("200 OK", r#"{"challenge":"abc.def","bits":40}"#.into()),
            ("200 OK", r#"{"challenge":"abc.def","bits":10}"#.into()),
            ("429 Too Many Requests", "{}".into()),
        ]);
        let id = "0b4a6c1e-1111-4222-8333-444455556666";
        let shared = share(&base, &code, id).unwrap();
        assert_eq!((shared.code.as_str(), shared.expires), ("ABCD2345", 1_790_000_000));
        let log = seen.lock().unwrap().clone();
        assert!(log[0].starts_with("GET /challenge"));
        assert!(log[1].starts_with("POST /"));
        assert!(log[1].contains(id) && log[1].contains("\"counter\""));
        assert_eq!(fetch(&base, "ABCD2345").unwrap(), code);
        assert!(matches!(fetch(&base, "ABCD2345"), Err(AppError::NotFound(_))));
        assert!(fetch(&base, "ABCD2345").is_err());
        assert!(fetch(&base, "abc").is_err());
        assert!(share(&base, "not a profile", id).is_err());
        assert!(share(&base, &code, id).is_err());
        assert!(matches!(share(&base, &code, id), Err(AppError::RateLimited)));
    }

    #[test]
    #[ignore = "talks to the real profile server"]
    fn live_share_and_fetch_against_the_real_server() {
        let r = sample();
        let code = encode(&build(&r, &names(&r), "live check").profile).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let shared = share(SERVER, &code, &id).unwrap();
        assert_eq!(fetch(SERVER, &shared.code).unwrap(), code);
    }
}
