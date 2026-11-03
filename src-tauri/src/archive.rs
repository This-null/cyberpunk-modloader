use crate::error::AppError;
use crate::library::Limits;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const RAR_FILE_CAP: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Zip,
    Rar,
    SevenZip,
}

pub fn kind(path: &Path) -> Option<Kind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "zip" => Some(Kind::Zip),
        "rar" => Some(Kind::Rar),
        "7z" => Some(Kind::SevenZip),
        _ => None,
    }
}

fn clean_name(raw: &str) -> Result<Option<String>, AppError> {
    let norm = raw.replace('\\', "/");
    let bytes = norm.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if norm.starts_with('/') || drive || norm.split('/').any(|s| s == "..") {
        return Err(AppError::UnsafePath(raw.to_string()));
    }
    let trimmed = norm.trim_matches('/');
    Ok(if trimmed.is_empty() { None } else { Some(trimmed.to_string()) })
}

struct Sink {
    zip: ZipWriter<fs::File>,
    limits: Limits,
    entries: usize,
    total: u64,
}

impl Sink {
    fn add(&mut self, raw: &str, data: &mut dyn Read, declared: Option<u64>) -> Result<(), AppError> {
        let Some(name) = clean_name(raw)? else {
            return Ok(());
        };
        self.entries += 1;
        if self.entries > self.limits.max_entries {
            return Err(AppError::Zip("the archive has too many entries".into()));
        }
        if declared.is_some_and(|d| d > self.limits.max_file) {
            return Err(AppError::Zip("a file in the archive is too large".into()));
        }
        let opts = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .large_file(true);
        self.zip.start_file(name, opts)?;
        let room = self.limits.max_total.saturating_sub(self.total).min(self.limits.max_file);
        let mut limited = data.take(room + 1);
        let written = std::io::copy(&mut limited, &mut self.zip)?;
        if written > room {
            return Err(AppError::Zip("the archive is too large once unpacked".into()));
        }
        self.total += written;
        Ok(())
    }
}

fn unpack_rar(src: &Path, sink: &mut Sink) -> Result<(), AppError> {
    let bad = |e: unrar::error::UnrarError| AppError::Zip(format!("could not read the rar file: {e}"));
    let mut archive = unrar::Archive::new(src).open_for_processing().map_err(bad)?;
    while let Some(header) = archive.read_header().map_err(bad)? {
        let entry = header.entry();
        if entry.is_encrypted() {
            return Err(AppError::Zip("password protected archives are not supported".into()));
        }
        if !entry.is_file() {
            archive = header.skip().map_err(bad)?;
            continue;
        }
        let name = entry.filename.to_string_lossy().into_owned();
        if entry.unpacked_size > RAR_FILE_CAP {
            return Err(AppError::Zip("a file in the rar archive is too large".into()));
        }
        let declared = entry.unpacked_size;
        let (data, next) = header.read().map_err(bad)?;
        sink.add(&name, &mut data.as_slice(), Some(declared))?;
        archive = next;
    }
    Ok(())
}

fn unpack_7z(src: &Path, sink: &mut Sink) -> Result<(), AppError> {
    let mut reader = sevenz_rust2::ArchiveReader::open(src, sevenz_rust2::Password::empty())
        .map_err(|e| AppError::Zip(format!("could not read the 7z file: {e}")))?;
    let mut failure: Option<AppError> = None;
    let result = reader.for_each_entries(|entry, data| {
        if entry.is_directory() || !entry.has_stream() {
            if !entry.is_directory() {
                if let Err(e) = sink.add(entry.name(), &mut std::io::empty(), Some(0)) {
                    failure = Some(e);
                    return Err(sevenz_rust2::Error::Other("stopped".into()));
                }
            }
            return Ok(true);
        }
        match sink.add(entry.name(), data, Some(entry.size())) {
            Ok(()) => Ok(true),
            Err(e) => {
                failure = Some(e);
                Err(sevenz_rust2::Error::Other("stopped".into()))
            }
        }
    });
    if let Some(e) = failure {
        return Err(e);
    }
    result.map_err(|e| AppError::Zip(format!("could not read the 7z file: {e}")))
}

pub fn to_zip(src: &Path, work: &Path, limits: Limits) -> Result<PathBuf, AppError> {
    let Some(k) = kind(src) else {
        return Err(AppError::Zip("only .zip, .rar and .7z files can be installed".into()));
    };
    if k == Kind::Zip {
        return Ok(src.to_path_buf());
    }
    fs::create_dir_all(work)?;
    let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "mod".into());
    let out = work.join(format!("{stem}.zip"));
    let mut sink = Sink {
        zip: ZipWriter::new(fs::File::create(&out)?),
        limits,
        entries: 0,
        total: 0,
    };
    let done = match k {
        Kind::Rar => unpack_rar(src, &mut sink),
        Kind::SevenZip => unpack_7z(src, &mut sink),
        Kind::Zip => Ok(()),
    };
    let finished = done.and_then(|()| sink.zip.finish().map(|_| ()).map_err(AppError::from));
    if let Err(e) = finished {
        let _ = fs::remove_file(&out);
        return Err(e);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn kinds_follow_the_extension_in_any_case() {
        assert_eq!(kind(Path::new("a/Mod.ZIP")), Some(Kind::Zip));
        assert_eq!(kind(Path::new("a/mod.rar")), Some(Kind::Rar));
        assert_eq!(kind(Path::new("a/mod.7Z")), Some(Kind::SevenZip));
        assert_eq!(kind(Path::new("a/mod.exe")), None);
        assert_eq!(kind(Path::new("a/mod")), None);
    }

    #[test]
    fn names_that_escape_are_refused() {
        assert!(matches!(clean_name("../x.dll"), Err(AppError::UnsafePath(_))));
        assert!(matches!(clean_name("/x"), Err(AppError::UnsafePath(_))));
        assert!(matches!(clean_name("C:/x"), Err(AppError::UnsafePath(_))));
        assert_eq!(clean_name("a\\b/c.txt").unwrap().as_deref(), Some("a/b/c.txt"));
        assert_eq!(clean_name("dir/").unwrap().as_deref(), Some("dir"));
        assert_eq!(clean_name("").unwrap(), None);
    }

    #[test]
    fn a_zip_passes_through_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("m.zip");
        fs::write(&p, b"x").unwrap();
        assert_eq!(to_zip(&p, &tmp.path().join("w"), Limits::default()).unwrap(), p);
    }

    #[test]
    fn other_files_are_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("m.exe");
        fs::write(&p, b"x").unwrap();
        assert!(to_zip(&p, &tmp.path().join("w"), Limits::default()).is_err());
    }

    #[test]
    fn a_7z_is_repacked_into_a_zip_with_the_same_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("in");
        fs::create_dir_all(src.join("archive/pc/mod")).unwrap();
        fs::write(src.join("archive/pc/mod/a.archive"), b"hello").unwrap();
        fs::write(src.join("readme.txt"), b"hi").unwrap();
        let seven = tmp.path().join("Cool Mod.7z");
        sevenz_rust2::compress_to_path(&src, &seven).unwrap();
        let out = to_zip(&seven, &tmp.path().join("w"), Limits::default()).unwrap();
        assert_eq!(out.file_name().unwrap(), "Cool Mod.zip");
        let mut z = zip::ZipArchive::new(fs::File::open(&out).unwrap()).unwrap();
        let mut got = String::new();
        z.by_name("archive/pc/mod/a.archive").unwrap().read_to_string(&mut got).unwrap();
        assert_eq!(got, "hello");
        assert!(z.by_name("readme.txt").is_ok());
    }

    #[test]
    fn a_real_rar_is_repacked_when_a_sample_is_given() {
        let Some(sample) = std::env::var_os("CP_RAR_SAMPLE") else {
            return;
        };
        let tmp = tempfile::tempdir().unwrap();
        let out = to_zip(Path::new(&sample), tmp.path(), Limits::default()).unwrap();
        let z = zip::ZipArchive::new(fs::File::open(out).unwrap()).unwrap();
        assert!(z.len() > 0);
    }

    #[test]
    fn a_broken_archive_leaves_nothing_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("w");
        for name in ["bad.rar", "bad.7z"] {
            let p = tmp.path().join(name);
            fs::File::create(&p).unwrap().write_all(b"not an archive").unwrap();
            assert!(to_zip(&p, &work, Limits::default()).is_err(), "{name}");
        }
        assert_eq!(fs::read_dir(&work).unwrap().count(), 0);
    }

    #[test]
    fn the_sink_enforces_entry_and_size_limits() {
        let tmp = tempfile::tempdir().unwrap();
        let limits = Limits { max_entries: 2, max_total: 10, max_file: 6 };
        let mut sink = Sink {
            zip: ZipWriter::new(fs::File::create(tmp.path().join("o.zip")).unwrap()),
            limits,
            entries: 0,
            total: 0,
        };
        assert!(sink.add("a.txt", &mut &b"12345"[..], Some(5)).is_ok());
        assert!(sink.add("b.txt", &mut &b"1234567"[..], None).is_err());
        let mut sink2 = Sink {
            zip: ZipWriter::new(fs::File::create(tmp.path().join("p.zip")).unwrap()),
            limits: Limits { max_entries: 1, max_total: 100, max_file: 100 },
            entries: 0,
            total: 0,
        };
        assert!(sink2.add("a", &mut &b"1"[..], None).is_ok());
        assert!(sink2.add("b", &mut &b"1"[..], None).is_err());
    }
}
