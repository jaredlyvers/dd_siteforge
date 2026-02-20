use std::fs;
use std::path::Path;

use anyhow::Context;

use crate::model::Site;

pub fn save_site<P: AsRef<Path>>(path: P, site: &Site) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(site).context("failed to serialize site to JSON")?;
    fs::write(path, json).context("failed to write site JSON")?;
    Ok(())
}

pub fn load_site<P: AsRef<Path>>(path: P) -> anyhow::Result<Site> {
    let json = fs::read_to_string(path).context("failed to read site JSON")?;
    let site = serde_json::from_str(&json).context("failed to parse site JSON")?;
    Ok(site)
}
