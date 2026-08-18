use std::fs;
use std::io;
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tier::{ConfigLoader, ReloadEvent, ReloadHandle};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    port: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { port: 3000 }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_file = tempfile::Builder::new().suffix(".toml").tempfile()?;
    let path = config_file.path().to_owned();
    fs::write(&path, "port = 3000\n")?;

    let path_for_loader = path.clone();
    let handle = ReloadHandle::new(move || {
        ConfigLoader::new(AppConfig::default())
            .file(path_for_loader.clone())
            .load()
    })?;

    let events = handle.subscribe();
    let watcher = handle.start_polling([path.clone()], Duration::from_millis(50));
    thread::sleep(Duration::from_millis(100));
    fs::write(&path, "port = 4000\n")?;

    match events.recv_timeout(Duration::from_secs(2))? {
        ReloadEvent::Applied(summary) if summary.had_changes => {}
        ReloadEvent::Applied(_) => {
            return Err(io::Error::other("reload event did not report changes").into());
        }
        ReloadEvent::Rejected(failure) => {
            return Err(io::Error::other(failure.error).into());
        }
    }

    println!("reloaded port = {}", handle.config().port);

    watcher.stop();
    Ok(())
}
