//! Bundled Handlebars templates + optional `<site>/source/templates/` overrides.
//!
//! Seed once with `init-site` or `init-templates`. Export never writes these
//! files. `init-templates --force` is the only overwrite.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context};
use handlebars::Handlebars;
use serde_json::Value;

pub const BUNDLED: &[(&str, &str)] = &[
    ("_page", include_str!("../templates/_page.hbs")),
    ("_head", include_str!("../templates/_head.hbs")),
    ("dd-header", include_str!("../templates/dd-header.hbs")),
    ("dd-footer", include_str!("../templates/dd-footer.hbs")),
    ("dd-hero", include_str!("../templates/dd-hero.hbs")),
    ("dd-section", include_str!("../templates/dd-section.hbs")),
    (
        "dd-section-column",
        include_str!("../templates/dd-section-column.hbs"),
    ),
    ("dd-alternating", include_str!("../templates/dd-alternating.hbs")),
    ("dd-card", include_str!("../templates/dd-card.hbs")),
    ("dd-banner", include_str!("../templates/dd-banner.hbs")),
    ("dd-cta", include_str!("../templates/dd-cta.hbs")),
    ("dd-filmstrip", include_str!("../templates/dd-filmstrip.hbs")),
    ("dd-milestones", include_str!("../templates/dd-milestones.hbs")),
    ("dd-modal", include_str!("../templates/dd-modal.hbs")),
    ("dd-slider", include_str!("../templates/dd-slider.hbs")),
    ("dd-accordion", include_str!("../templates/dd-accordion.hbs")),
    ("dd-blockquote", include_str!("../templates/dd-blockquote.hbs")),
    ("dd-alert", include_str!("../templates/dd-alert.hbs")),
    ("dd-image", include_str!("../templates/dd-image.hbs")),
    ("dd-rich_text", include_str!("../templates/dd-rich_text.hbs")),
    ("dd-navigation", include_str!("../templates/dd-navigation.hbs")),
    (
        "dd-header-search",
        include_str!("../templates/dd-header-search.hbs"),
    ),
    (
        "dd-header-menu",
        include_str!("../templates/dd-header-menu.hbs"),
    ),
];

pub fn bundled(name: &str) -> Option<&'static str> {
    BUNDLED.iter().find(|(n, _)| *n == name).map(|(_, src)| *src)
}

pub fn templates_dir(site_root: &Path) -> std::path::PathBuf {
    site_root.join("source").join("templates")
}

pub struct Renderer {
    hbs: Handlebars<'static>,
}

impl Renderer {
    pub fn load(site_root: Option<&Path>) -> anyhow::Result<Self> {
        let mut sources: HashMap<&str, String> = HashMap::new();
        for (name, src) in BUNDLED {
            sources.insert(*name, (*src).to_string());
        }
        if let Some(root) = site_root {
            let dir = templates_dir(root);
            if dir.is_dir() {
                for (name, _) in BUNDLED {
                    let path = dir.join(format!("{name}.hbs"));
                    if path.exists() {
                        let text = fs::read_to_string(&path).with_context(|| {
                            format!("failed to read override '{}'", path.display())
                        })?;
                        sources.insert(*name, text);
                    }
                }
            }
        }

        let mut hbs = Handlebars::new();
        for (name, src) in &sources {
            hbs.register_template_string(*name, src).with_context(|| {
                format!("failed to parse template '{name}.hbs'")
            })?;
        }
        Ok(Self { hbs })
    }

    #[cfg(test)]
    pub fn bundled_only() -> anyhow::Result<Self> {
        Self::load(None)
    }

    pub fn render(&self, name: &str, data: &Value) -> anyhow::Result<String> {
        self.hbs
            .render(name, data)
            .with_context(|| format!("failed to render template '{name}'"))
    }
}

pub struct SeedReport {
    pub written: Vec<String>,
    pub skipped: Vec<String>,
}

/// Write bundled templates into `<site_root>/source/templates/`.
/// Existing files are left alone unless `force` is set.
pub fn seed_templates(
    site_root: &Path,
    force: bool,
    only: Option<&str>,
) -> anyhow::Result<SeedReport> {
    if let Some(name) = only {
        if bundled(name).is_none() {
            return Err(anyhow!(
                "unknown template '{name}'. Known: {}",
                BUNDLED
                    .iter()
                    .map(|(n, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    let dir = templates_dir(site_root);
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create '{}'", dir.display()))?;
    let mut report = SeedReport {
        written: Vec::new(),
        skipped: Vec::new(),
    };
    for (name, src) in BUNDLED {
        if only.is_some_and(|n| n != *name) {
            continue;
        }
        let path = dir.join(format!("{name}.hbs"));
        if path.exists() && !force {
            report.skipped.push((*name).to_string());
            continue;
        }
        fs::write(&path, src)
            .with_context(|| format!("failed to write '{}'", path.display()))?;
        report.written.push((*name).to_string());
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp() -> std::path::PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("dd_tpl_{n}"))
    }

    #[test]
    fn bundled_renderer_parses_every_template() {
        let r = Renderer::bundled_only().expect("bundled templates must parse");
        let html = r
            .render("_page", &json!({"lang":"en","head_html":"","header_html":"","footer_html":"","content":"hi"}))
            .unwrap();
        assert!(html.contains("<main>"));
        assert!(html.contains("hi"));
    }

    #[test]
    fn seed_skips_existing_unless_force() {
        let root = tmp();
        let first = seed_templates(&root, false, None).unwrap();
        assert!(!first.written.is_empty());
        assert!(first.skipped.is_empty());
        let path = templates_dir(&root).join("dd-hero.hbs");
        fs::write(&path, "OVERRIDE").unwrap();
        let second = seed_templates(&root, false, None).unwrap();
        assert!(second.written.is_empty());
        assert!(second.skipped.contains(&"dd-hero".to_string()));
        assert_eq!(fs::read_to_string(&path).unwrap(), "OVERRIDE");
        seed_templates(&root, true, Some("dd-hero")).unwrap();
        assert_ne!(fs::read_to_string(&path).unwrap(), "OVERRIDE");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn override_file_wins_and_broken_hbs_errors() {
        let root = tmp();
        seed_templates(&root, false, Some("dd-alert")).unwrap();
        let path = templates_dir(&root).join("dd-alert.hbs");
        fs::write(&path, "<div>ok {{parent_copy}}</div>").unwrap();
        let r = Renderer::load(Some(&root)).unwrap();
        let html = r
            .render(
                "dd-alert",
                &json!({
                    "parent_type": "-default",
                    "parent_class": "-default",
                    "sal": "fade",
                    "parent_title": "",
                    "has_title": false,
                    "parent_copy": "hello"
                }),
            )
            .unwrap();
        assert!(html.contains("hello"));
        fs::write(&path, "{{#if unterminated").unwrap();
        let err = match Renderer::load(Some(&root)) {
            Err(e) => e,
            Ok(_) => panic!("expected parse error"),
        };
        let msg = format!("{err:#}");
        assert!(msg.contains("dd-alert.hbs"), "{msg}");
        fs::remove_dir_all(&root).ok();
    }
}
