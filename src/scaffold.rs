//! Bundled build kit (Grunt, source/, Lando, DDEV) seeded on init.
//!
//! Canonical copy lives in the binary. `~/.config/ldnddev/dd_siteforge/` is an
//! optional per-file overlay. Existing project files are left alone unless
//! `force`. Export never writes these files.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use rust_embed::RustEmbed;

const GRUNTFILE: &str = include_str!("../Gruntfile.js");
const PACKAGE_JSON: &str = include_str!("../package.json");
const LANDO_YML: &str = include_str!("../.lando.yml");
const DDEV_CONFIG: &str = include_str!("../.ddev/config.yaml");
const IMAGES_GITKEEP: &[u8] = include_bytes!("../source/images/.gitkeep");

#[derive(RustEmbed)]
#[folder = "source/"]
#[exclude = "images/*"]
#[exclude = "images/**"]
#[exclude = "templates/*"]
#[exclude = "templates/**"]
struct BundledSource;

pub struct SeedOpts<'a> {
    pub force: bool,
    pub project_name: Option<&'a str>,
    pub overlay: Option<&'a Path>,
}

#[derive(Default)]
pub struct SeedReport {
    pub written: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn config_scaffold_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| Path::new(&h).join(".config"))
        })
        .unwrap_or_else(|| PathBuf::from(".config"));
    base.join("ldnddev").join("dd_siteforge")
}

pub fn overlay_if_present() -> Option<PathBuf> {
    let dir = config_scaffold_dir();
    dir.is_dir().then_some(dir)
}

pub fn slugify_project_name(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for c in raw.chars() {
        let mapped = match c.to_ascii_lowercase() {
            c @ 'a'..='z' | c @ '0'..='9' => Some(c),
            ' ' | '_' | '-' => Some('-'),
            _ => None,
        };
        match mapped {
            Some('-') => {
                if !out.is_empty() && !prev_hyphen {
                    out.push('-');
                    prev_hyphen = true;
                }
            }
            Some(ch) => {
                out.push(ch);
                prev_hyphen = false;
            }
            None => {}
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() > 63 {
        out.truncate(63);
        while out.ends_with('-') {
            out.pop();
        }
    }
    if out.is_empty() {
        "dd-siteforge".to_string()
    } else {
        out
    }
}

pub fn dir_hint(root: &Path) -> String {
    root.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
        .or_else(|| {
            root.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty() && s != ".")
        })
        .unwrap_or_else(|| "dd-siteforge".to_string())
}

pub fn seed_scaffold(dest: &Path, opts: SeedOpts<'_>) -> anyhow::Result<SeedReport> {
    let mut files = bundled_files();
    if let Some(overlay) = opts.overlay {
        merge_overlay(overlay, &mut files)?;
    }

    let mut report = SeedReport::default();
    for (rel, bytes) in files {
        let path = dest.join(Path::new(&rel));
        if path.exists() && !opts.force {
            report.skipped.push(rel);
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        let payload = match (opts.project_name, identity_kind(&rel)) {
            (Some(slug), Some(kind)) => stamp_identity(kind, &bytes, slug)?,
            _ => bytes,
        };
        fs::write(&path, payload)
            .with_context(|| format!("failed to write '{}'", path.display()))?;
        report.written.push(rel);
    }
    Ok(report)
}

fn bundled_files() -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    files.insert("Gruntfile.js".into(), GRUNTFILE.as_bytes().to_vec());
    files.insert("package.json".into(), PACKAGE_JSON.as_bytes().to_vec());
    files.insert(".lando.yml".into(), LANDO_YML.as_bytes().to_vec());
    files.insert(".ddev/config.yaml".into(), DDEV_CONFIG.as_bytes().to_vec());
    files.insert("source/images/.gitkeep".into(), IMAGES_GITKEEP.to_vec());
    for path in BundledSource::iter() {
        let rel = format!("source/{path}");
        if skip_rel(&rel) {
            continue;
        }
        if let Some(file) = BundledSource::get(path.as_ref()) {
            files.insert(rel, file.data.into_owned());
        }
    }
    files
}

fn merge_overlay(overlay: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> anyhow::Result<()> {
    let mut extra = Vec::new();
    walk_files(overlay, overlay, &mut extra)?;
    for (rel, path) in extra {
        if skip_rel(&rel) {
            continue;
        }
        let bytes = fs::read(&path)
            .with_context(|| format!("failed to read overlay '{}'", path.display()))?;
        files.insert(rel, bytes);
    }
    Ok(())
}

fn walk_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> anyhow::Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read overlay '{}'", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            walk_files(root, &path, out)?;
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push((rel, path));
    }
    Ok(())
}

fn skip_rel(rel: &str) -> bool {
    let r = rel.replace('\\', "/");
    if r == "source/templates" || r.starts_with("source/templates/") {
        return true;
    }
    if r.starts_with("source/images/") && r != "source/images/.gitkeep" {
        return true;
    }
    false
}

#[derive(Clone, Copy)]
enum IdentityKind {
    PackageJson,
    Lando,
    Ddev,
}

fn identity_kind(rel: &str) -> Option<IdentityKind> {
    match rel.replace('\\', "/").as_str() {
        "package.json" => Some(IdentityKind::PackageJson),
        ".lando.yml" => Some(IdentityKind::Lando),
        ".ddev/config.yaml" => Some(IdentityKind::Ddev),
        _ => None,
    }
}

fn stamp_identity(kind: IdentityKind, bytes: &[u8], slug: &str) -> anyhow::Result<Vec<u8>> {
    let text = std::str::from_utf8(bytes).map_err(|_| anyhow!("identity file is not UTF-8"))?;
    let stamped = match kind {
        IdentityKind::PackageJson => stamp_package_json(text, slug)?,
        IdentityKind::Lando => stamp_yaml_name_and_lndo(text, slug),
        IdentityKind::Ddev => stamp_yaml_name(text, slug),
    };
    Ok(stamped.into_bytes())
}

fn stamp_package_json(text: &str, slug: &str) -> anyhow::Result<String> {
    let mut v: serde_json::Value =
        serde_json::from_str(text).context("overlay package.json is not valid JSON")?;
    match &mut v {
        serde_json::Value::Object(map) => {
            map.insert("name".into(), serde_json::Value::String(slug.to_string()));
        }
        _ => return Err(anyhow!("package.json root must be an object")),
    }
    Ok(serde_json::to_string_pretty(&v)? + "\n")
}

fn stamp_yaml_name(text: &str, slug: &str) -> String {
    let mut out = String::new();
    let mut named = false;
    for chunk in split_inclusive_newline(text) {
        let (body, nl) = chunk;
        if !named && is_top_level_name_key(body) {
            out.push_str("name: ");
            out.push_str(slug);
            out.push_str(nl);
            named = true;
        } else {
            out.push_str(body);
            out.push_str(nl);
        }
    }
    if !named {
        format!("name: {slug}\n{out}")
    } else {
        out
    }
}

fn stamp_yaml_name_and_lndo(text: &str, slug: &str) -> String {
    let named = stamp_yaml_name(text, slug);
    replace_lndo_hosts(&named, slug)
}

fn is_top_level_name_key(line: &str) -> bool {
    let line = line.trim_end_matches('\r');
    let trimmed = line.trim_start();
    if trimmed.len() != line.len() {
        return false;
    }
    trimmed.starts_with("name:") || trimmed.starts_with("name :")
}

fn replace_lndo_hosts(text: &str, slug: &str) -> String {
    let needle = ".lndo.site";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(needle) {
        let prefix = &rest[..idx];
        let host_start = prefix
            .char_indices()
            .rev()
            .find(|(_, c)| !is_hostname_char(*c))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        out.push_str(&prefix[..host_start]);
        out.push_str(slug);
        out.push_str(needle);
        rest = &rest[idx + needle.len()..];
    }
    out.push_str(rest);
    out
}

fn is_hostname_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '.'
}

fn split_inclusive_newline(text: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find('\n') {
        out.push((&rest[..idx], "\n"));
        rest = &rest[idx + 1..];
    }
    if !rest.is_empty() {
        out.push((rest, ""));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(prefix: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("dd_scaf_{prefix}_{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn seed(dest: &Path, name: Option<&str>, overlay: Option<&Path>, force: bool) -> SeedReport {
        seed_scaffold(
            dest,
            SeedOpts {
                force,
                project_name: name,
                overlay,
            },
        )
        .unwrap()
    }

    #[test]
    fn embed_excludes_images_and_templates() {
        for path in BundledSource::iter() {
            assert!(
                !path.starts_with("images/"),
                "user images must not be embedded: {path}"
            );
            assert!(
                !path.starts_with("templates/"),
                "handlebars stay on the templates seed path: {path}"
            );
        }
    }

    #[test]
    fn slugify_examples() {
        assert_eq!(slugify_project_name("My Cool Site!"), "my-cool-site");
        assert_eq!(slugify_project_name("Acme_Site"), "acme-site");
        assert_eq!(slugify_project_name("  --Foo--  "), "foo");
        assert_eq!(slugify_project_name(""), "dd-siteforge");
        assert_eq!(slugify_project_name("***"), "dd-siteforge");
        assert_eq!(slugify_project_name("dd-siteforge"), "dd-siteforge");
        let long = "a".repeat(80);
        assert_eq!(slugify_project_name(&long).len(), 63);
    }

    #[test]
    fn seed_writes_kit_and_stamps_names() {
        let root = tmp("write");
        let report = seed(&root, Some("acme-site"), None, false);
        assert!(report.skipped.is_empty());
        assert!(report.written.iter().any(|p| p == "Gruntfile.js"));
        assert!(report.written.iter().any(|p| p == "package.json"));
        assert!(report.written.iter().any(|p| p == ".lando.yml"));
        assert!(report.written.iter().any(|p| p == ".ddev/config.yaml"));
        assert!(report.written.iter().any(|p| p == "source/js/main.js"));
        assert!(report.written.iter().any(|p| p == "source/images/.gitkeep"));
        assert!(report.written.iter().any(|p| p.ends_with(".woff2")));
        assert!(!report.written.iter().any(|p| p.starts_with("source/templates/")));
        assert!(!root.join("source/templates").exists());

        let pkg: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
        assert_eq!(pkg["name"], "acme-site");

        let lando = fs::read_to_string(root.join(".lando.yml")).unwrap();
        assert!(lando.contains("name: acme-site"), "{lando}");
        assert!(lando.contains("acme-site.lndo.site"), "{lando}");
        assert!(!lando.contains("dd-siteforge.lndo.site"), "{lando}");

        let ddev = fs::read_to_string(root.join(".ddev/config.yaml")).unwrap();
        assert!(ddev.contains("name: acme-site"), "{ddev}");

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn seed_skips_existing_and_does_not_restamp() {
        let root = tmp("skip");
        seed(&root, Some("first"), None, false);
        fs::write(root.join("package.json"), "{\n  \"name\": \"keep-me\"\n}\n").unwrap();
        fs::write(root.join(".lando.yml"), "name: keep-me\n").unwrap();
        let second = seed(&root, Some("second"), None, false);
        assert!(second.written.is_empty());
        assert!(second.skipped.contains(&"package.json".to_string()));
        let pkg = fs::read_to_string(root.join("package.json")).unwrap();
        assert!(pkg.contains("keep-me"));
        let lando = fs::read_to_string(root.join(".lando.yml")).unwrap();
        assert_eq!(lando, "name: keep-me\n");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn seed_force_overwrites_and_stamps() {
        let root = tmp("force");
        seed(&root, Some("first"), None, false);
        fs::write(root.join("package.json"), "{\n  \"name\": \"old\"\n}\n").unwrap();
        seed(&root, Some("fresh-name"), None, true);
        let pkg: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
        assert_eq!(pkg["name"], "fresh-name");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn overlay_wins_and_extra_file_copies_images_and_templates_skipped() {
        let overlay = tmp("overlay");
        fs::create_dir_all(overlay.join("source/js")).unwrap();
        fs::write(overlay.join("Gruntfile.js"), "/* house */\n").unwrap();
        fs::write(overlay.join("source/js/extra.js"), "console.log('x');\n").unwrap();
        fs::create_dir_all(overlay.join("source/images")).unwrap();
        fs::write(overlay.join("source/images/secret.png"), b"nope").unwrap();
        fs::create_dir_all(overlay.join("source/templates")).unwrap();
        fs::write(overlay.join("source/templates/dd-hero.hbs"), "NO").unwrap();

        let root = tmp("overdest");
        let report = seed(&root, Some("house"), Some(&overlay), false);
        assert_eq!(
            fs::read_to_string(root.join("Gruntfile.js")).unwrap(),
            "/* house */\n"
        );
        assert!(root.join("source/js/extra.js").exists());
        assert!(report.written.iter().any(|p| p == "source/js/extra.js"));
        assert!(!root.join("source/images/secret.png").exists());
        assert!(!root.join("source/templates/dd-hero.hbs").exists());
        fs::remove_dir_all(&overlay).ok();
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn global_seed_does_not_stamp_names() {
        let dest = tmp("global");
        seed(&dest, None, None, false);
        let pkg: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dest.join("package.json")).unwrap()).unwrap();
        assert_eq!(pkg["name"], "dd_siteforge");
        let lando = fs::read_to_string(dest.join(".lando.yml")).unwrap();
        assert!(lando.contains("name: dd-siteforge"), "{lando}");
        fs::remove_dir_all(&dest).ok();
    }

    #[test]
    fn replace_lndo_hosts_rewrites_comments_and_proxy() {
        let src = "name: dd-siteforge\n# https://dd-siteforge.lndo.site\n    - dd-siteforge.lndo.site\n";
        let out = stamp_yaml_name_and_lndo(src, "acme");
        assert!(out.contains("name: acme"));
        assert!(out.contains("https://acme.lndo.site"));
        assert!(out.contains("- acme.lndo.site"));
        assert!(!out.contains("dd-siteforge"));
    }
}
