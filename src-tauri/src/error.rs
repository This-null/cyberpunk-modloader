use crate::conflicts::Conflict;
use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unsafe path in archive: {0}")]
    UnsafePath(String),
    #[error("cannot tell where this file belongs: {0}")]
    Unmappable(String),
    #[error("this archive holds no mod files: {0}")]
    NoFiles(String),
    #[error("this mod uses an installer with options (FOMOD)")]
    Fomod,
    #[error("these files belong to an installed mod: {0}")]
    Owned(String),
    #[error("close Cyberpunk 2077 before changing mods")]
    GameRunning,
    #[error("Cyberpunk 2077 folder not set")]
    GameNotFound,
    #[error("file error: {0}")]
    Io(String),
    #[error("archive error: {0}")]
    Zip(String),
    #[error("state file error: {0}")]
    State(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid mod name: {0}")]
    InvalidName(String),
    #[error("some files already belong to other mods")]
    Conflict(Vec<Conflict>),
    #[error("network error: {0}")]
    Network(String),
    #[error("the downloaded file does not match its checksum")]
    Checksum,
    #[error("release not found: {0}")]
    ReleaseNotFound(String),
    #[error("release withdrawn: {0}")]
    Withdrawn(String),
    #[error("Nexus did not accept the API key")]
    NexusKey,
    #[error("Nexus asked us to slow down")]
    RateLimited,
    #[error("not a Cyberpunk 2077 mod link: {0}")]
    BadLink(String),
    #[error("one-click install needs a Nexus Premium account")]
    NexusPremium,
    #[error("some files are in use: {0}")]
    FilesInUse(String),
    #[error("the Nexus download link expired")]
    NexusLinkExpired,
    #[error("download host not trusted: {0}")]
    UntrustedHost(String),
    #[error("disable your mods before switching the game folder")]
    ModsEnabled,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::UnsafePath(_) => "unsafe_path",
            AppError::Unmappable(_) => "unmappable",
            AppError::NoFiles(_) => "no_files",
            AppError::Fomod => "fomod",
            AppError::Owned(_) => "owned",
            AppError::GameRunning => "game_running",
            AppError::GameNotFound => "game_not_found",
            AppError::Io(_) => "io",
            AppError::Zip(_) => "zip",
            AppError::State(_) => "state",
            AppError::NotFound(_) => "not_found",
            AppError::InvalidName(_) => "invalid_name",
            AppError::Conflict(_) => "conflict",
            AppError::Network(_) => "network",
            AppError::Checksum => "checksum",
            AppError::ReleaseNotFound(_) => "release_not_found",
            AppError::Withdrawn(_) => "release_withdrawn",
            AppError::NexusKey => "nexus_key",
            AppError::RateLimited => "rate_limited",
            AppError::BadLink(_) => "bad_link",
            AppError::NexusPremium => "nexus_premium",
            AppError::FilesInUse(_) => "files_in_use",
            AppError::NexusLinkExpired => "nexus_link_expired",
            AppError::UntrustedHost(_) => "download_host",
            AppError::ModsEnabled => "mods_enabled",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("AppError", 3)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        match self {
            AppError::Conflict(list) => st.serialize_field("details", list)?,
            _ => st.serialize_field("details", &Option::<()>::None)?,
        }
        st.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<zip::result::ZipError> for AppError {
    fn from(e: zip::result::ZipError) -> Self {
        AppError::Zip(e.to_string())
    }
}
