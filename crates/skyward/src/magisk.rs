//! Magisk, KernelSU and APatch modules as they lie under `/data/adb`.

use anyhow::{Context, Result};
use skyward_ui::magisk::{MagiskModule, MagiskRoot};
use std::fs;
use std::path::Path;

const MODULE_DIRS: [&str; 3] = ["/data/adb/modules", "/data/adb/ksu/modules", "/data/adb/ap/modules"];

pub fn read() -> (MagiskRoot, Result<Vec<MagiskModule>>) {
    if !Path::new("/data").exists() {
        return (MagiskRoot::Locked, Ok(Vec::new()));
    }
    let root = if Path::new("/data/adb/magisk/magisk").exists() {
        Some("Magisk")
    } else if Path::new("/data/adb/ksud").exists() {
        Some("KernelSU")
    } else if Path::new("/data/adb/apd").exists() {
        Some("APatch")
    } else {
        None
    };
    let mut modules = Vec::new();
    for dir in MODULE_DIRS {
        let dir = Path::new(dir);
        if !dir.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => return (root.map_or(MagiskRoot::Absent, |r| MagiskRoot::Present(r.into())), Err(e.into())),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(module) = module_at(&path) else { continue };
            if modules.iter().any(|m: &MagiskModule| m.id == module.id) {
                continue;
            }
            modules.push(module);
        }
    }
    modules.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let root = match root {
        Some(name) => MagiskRoot::Present(name.into()),
        None => MagiskRoot::Absent,
    };
    (root, Ok(modules))
}

fn module_at(path: &Path) -> Option<MagiskModule> {
    let prop = fs::read_to_string(path.join("module.prop")).ok()?;
    let mut id = None;
    let mut name = None;
    let mut version = String::new();
    let mut author = String::new();
    let mut description = String::new();
    for line in prop.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim();
        match key.trim() {
            "id" => id = Some(value.to_string()),
            "name" => name = Some(value.to_string()),
            "version" => version = value.to_string(),
            "author" => author = value.to_string(),
            "description" => description = value.to_string(),
            _ => {}
        }
    }
    Some(MagiskModule {
        id: id?,
        name: name.unwrap_or_else(|| path.file_name().unwrap_or_default().to_string_lossy().into_owned()),
        version,
        author,
        description,
        enabled: !path.join("disable").exists(),
        removing: path.join("remove").exists(),
        path: path.to_string_lossy().into_owned(),
    })
}

pub fn toggle(path: &str, enabled: bool) -> Result<()> {
    let path = Path::new(path);
    let disable = path.join("disable");
    if enabled {
        if disable.exists() {
            fs::remove_file(&disable).with_context(|| format!("remove {}", disable.display()))?;
        }
    } else if !disable.exists() {
        fs::write(&disable, b"").with_context(|| format!("write {}", disable.display()))?;
    }
    Ok(())
}

pub fn remove(path: &str) -> Result<()> {
    let path = Path::new(path);
    let remove = path.join("remove");
    fs::write(&remove, b"").with_context(|| format!("write {}", remove.display()))?;
    Ok(())
}
