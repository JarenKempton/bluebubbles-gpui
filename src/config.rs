use std::{fs, io::Write, os::unix::fs::OpenOptionsExt, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub server_url: String,
    pub password: String,
}

fn path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bluebubbles-gpui")
        .join("config.json")
}

pub fn load() -> Option<Config> {
    serde_json::from_slice(&fs::read(path()).ok()?).ok()
}

pub fn save(config: &Config) -> Result<()> {
    let path = path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(&serde_json::to_vec_pretty(config)?)?;
    Ok(())
}

pub fn clear() {
    let _ = fs::remove_file(path());
}
