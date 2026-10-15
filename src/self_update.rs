use std::{fs, path::PathBuf, process::Command, time::SystemTime};

pub struct InstalledBinary {
    path: PathBuf,
    started_from: Option<SystemTime>,
}

impl InstalledBinary {
    pub fn current() -> Option<Self> {
        let path = std::env::current_exe().ok()?;
        let started_from = modified(&path);
        Some(Self { path, started_from })
    }

    pub fn replaced(&self) -> bool {
        let now = modified(&self.path);
        now.is_some() && now != self.started_from
    }

    pub fn relaunch(&self) -> bool {
        Command::new(&self.path).spawn().is_ok()
    }
}

fn modified(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}
