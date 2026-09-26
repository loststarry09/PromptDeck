use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "PromptDeck";
pub const DATA_DIR_ENV: &str = "PROMPTDECK_DATA_DIR";
pub const PORTABLE_FLAG: &str = "portable.flag";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDirSource {
    EnvOverride,
    Portable,
    PlatformDefault,
}

impl fmt::Display for DataDirSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            DataDirSource::EnvOverride => "env override",
            DataDirSource::Portable => "portable",
            DataDirSource::PlatformDefault => "platform default",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDir {
    pub path: PathBuf,
    pub source: DataDirSource,
}

pub fn resolve_data_dir() -> io::Result<DataDir> {
    let env_override = std::env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty());
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let portable = exe_dir
        .as_deref()
        .is_some_and(|dir| dir.join(PORTABLE_FLAG).is_file());

    resolve(env_override, exe_dir, portable, platform_data_dir())
}

fn resolve(
    env_override: Option<std::ffi::OsString>,
    exe_dir: Option<PathBuf>,
    portable: bool,
    platform_default: Option<PathBuf>,
) -> io::Result<DataDir> {
    if let Some(dir) = env_override {
        return Ok(DataDir {
            path: PathBuf::from(dir),
            source: DataDirSource::EnvOverride,
        });
    }

    if portable && let Some(dir) = exe_dir {
        return Ok(DataDir {
            path: dir.join("data"),
            source: DataDirSource::Portable,
        });
    }

    platform_default
        .map(|path| DataDir {
            path,
            source: DataDirSource::PlatformDefault,
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "cannot determine a platform data directory",
            )
        })
}

#[cfg(target_os = "windows")]
fn platform_data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| PathBuf::from(base).join(APP_DIR_NAME))
}

#[cfg(target_os = "macos")]
fn platform_data_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join("Library/Application Support")
            .join(APP_DIR_NAME)
    })
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn platform_data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(xdg).join(APP_DIR_NAME.to_lowercase()));
    }
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".local/share")
            .join(APP_DIR_NAME.to_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn default_dir() -> PathBuf {
        PathBuf::from("/home/user/.local/share/promptdeck")
    }

    #[test]
    fn env_override_wins_over_everything() {
        let resolved = resolve(
            Some(OsString::from("/tmp/promptdeck-data")),
            Some(PathBuf::from("/opt/promptdeck")),
            true,
            Some(default_dir()),
        )
        .expect("resolve");

        assert_eq!(resolved.source, DataDirSource::EnvOverride);
        assert_eq!(resolved.path, PathBuf::from("/tmp/promptdeck-data"));
    }

    #[test]
    fn portable_flag_uses_data_next_to_exe() {
        let resolved = resolve(
            None,
            Some(PathBuf::from("/opt/promptdeck")),
            true,
            Some(default_dir()),
        )
        .expect("resolve");

        assert_eq!(resolved.source, DataDirSource::Portable);
        assert_eq!(resolved.path, PathBuf::from("/opt/promptdeck/data"));
    }

    #[test]
    fn falls_back_to_platform_default() {
        let resolved = resolve(
            None,
            Some(PathBuf::from("/opt/promptdeck")),
            false,
            Some(default_dir()),
        )
        .expect("resolve");

        assert_eq!(resolved.source, DataDirSource::PlatformDefault);
        assert_eq!(resolved.path, default_dir());
    }

    #[test]
    fn errors_when_no_source_is_available() {
        let result = resolve(None, None, false, None);

        assert!(result.is_err());
    }
}
