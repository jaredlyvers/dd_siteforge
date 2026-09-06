use std::fs;
use std::path::Path;

use anyhow::Context;

use crate::model::{
    absolute_url, page_file_name, page_href, DdRichText, DdSection, Page, PageNode, RobotsDirective,
    SchemaType, SectionClass, SectionColumn, SectionComponent, SectionItemBoxClass, Site,
};
use crate::renderer::render_site_to_dir;

const GRUNT_ASSET_DIRS: &[&str] = &["css", "js", "webfonts", "favicon", "vendors"];

pub struct ExportReport {
    pub pages: usize,
    pub wrote_404: bool,
}

pub fn export_site(
    site: &Site,
    output_dir: &Path,
    site_root: Option<&Path>,
) -> anyhow::Result<ExportReport> {
    fs::create_dir_all(output_dir).context("failed to create export directory")?;
    render_site_to_dir(site, output_dir, site_root)?;
    copy_built_assets(site_root, output_dir)?;
    copy_source_images(site_root, output_dir)?;
    write_sitemap(site, output_dir)?;
    write_robots(site, output_dir)?;
    let wrote_404 = write_404_if_missing(site, output_dir, site_root)?;
    Ok(ExportReport {
        pages: site.pages.len(),
        wrote_404,
    })
}

/// Copy Grunt output (`<site>/web/assets/{css,js,webfonts,favicon,vendors}`)
/// into the export dir. When exporting *to* `web/`, skip that copy so a
/// local `grunt build` is not overwritten. Then fill missing webfonts /
/// favicon from `source/`.
fn copy_built_assets(site_root: Option<&Path>, output_dir: &Path) -> anyhow::Result<()> {
    let Some(root) = site_root else {
        return Ok(());
    };
    let dest_assets = output_dir.join("assets");
    let grunt_assets = root.join("web").join("assets");

    if grunt_assets.exists() && !same_assets_dir(&grunt_assets, &dest_assets) {
        for dir in GRUNT_ASSET_DIRS {
            let src = grunt_assets.join(dir);
            if src.exists() {
                copy_dir_recursive(&src, &dest_assets.join(dir)).with_context(|| {
                    format!("failed to copy grunt assets from '{}'", src.display())
                })?;
            }
        }
    }

    let source = root.join("source");
    fill_dir_if_empty(&source.join("webfonts"), &dest_assets.join("webfonts"))?;
    fill_dir_if_empty(&source.join("favicon"), &dest_assets.join("favicon"))?;
    Ok(())
}

fn same_assets_dir(a: &Path, b: &Path) -> bool {
    if let (Ok(ca), Ok(cb)) = (a.canonicalize(), b.canonicalize()) {
        return ca == cb;
    }
    a == b
}

fn fill_dir_if_empty(src: &Path, dest: &Path) -> anyhow::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if dest_has_files(dest) {
        return Ok(());
    }
    copy_dir_recursive(src, dest)
        .with_context(|| format!("failed to copy '{}'", src.display()))
}

fn dest_has_files(dir: &Path) -> bool {
    fs::read_dir(dir)
        .ok()
        .map(|mut it| it.next().is_some())
        .unwrap_or(false)
}

pub fn copy_source_images(site_root: Option<&Path>, output_dir: &Path) -> anyhow::Result<()> {
    let Some(root) = site_root else {
        return Ok(());
    };
    let src = root.join("source").join("images");
    if !src.exists() {
        return Ok(());
    }
    let dst = output_dir.join("assets").join("images");
    copy_dir_recursive(&src, &dst)
        .with_context(|| format!("failed to copy images from '{}'", src.display()))
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn write_sitemap(site: &Site, output_dir: &Path) -> anyhow::Result<()> {
    let mut body = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#,
    );
    for page in &site.pages {
        if matches!(
            page.head.robots,
            RobotsDirective::NoindexFollow | RobotsDirective::NoindexNofollow
        ) {
            continue;
        }
        let file = page_href(&page.slug);
        let loc = absolute_url(site.base_url.as_deref(), &file).unwrap_or(file);
        body.push_str("  <url><loc>");
        body.push_str(&xml_escape(&loc));
        body.push_str("</loc></url>\n");
    }
    body.push_str("</urlset>\n");
    fs::write(output_dir.join("sitemap.xml"), body).context("failed to write sitemap.xml")
}

fn write_robots(site: &Site, output_dir: &Path) -> anyhow::Result<()> {
    let mut body = String::from("User-agent: *\nAllow: /\n");
    if let Some(sitemap) = absolute_url(site.base_url.as_deref(), "sitemap.xml") {
        body.push('\n');
        body.push_str("Sitemap: ");
        body.push_str(&sitemap);
        body.push('\n');
    }
    fs::write(output_dir.join("robots.txt"), body).context("failed to write robots.txt")
}

fn write_404_if_missing(
    site: &Site,
    output_dir: &Path,
    site_root: Option<&Path>,
) -> anyhow::Result<bool> {
    if site.pages.iter().any(|p| p.slug == "404") {
        return Ok(false);
    }
    let page = not_found_page();
    let r = crate::templates::Renderer::load(site_root)?;
    let html = crate::renderer::render_page_html_with_chrome(
        &r,
        &page,
        &crate::renderer::render_header(&r, &site.header)?,
        &crate::renderer::render_footer(&r, &site.footer)?,
        site,
    )?;
    fs::write(output_dir.join(page_file_name("404")), html).context("failed to write 404.html")?;
    Ok(true)
}

fn not_found_page() -> Page {
    Page {
        id: "page-404".to_string(),
        slug: "404".to_string(),
        slug_locked: true,
        head: crate::model::DdHead {
            title: "Not Found".to_string(),
            meta_title: None,
            meta_description: Some("This page does not exist.".to_string()),
            canonical_url: None,
            robots: RobotsDirective::NoindexNofollow,
            schema_type: SchemaType::WebPage,
            og_title: None,
            og_description: None,
            og_image: None,
        },
        nodes: vec![PageNode::Section(DdSection {
            id: "section-404".to_string(),
            section_title: Some("Not Found".to_string()),
            section_class: Some(SectionClass::FullContained),
            item_box_class: Some(SectionItemBoxClass::LBox),
            columns: vec![SectionColumn {
                id: "column-1".to_string(),
                width_class: "dd-u-1-1".to_string(),
                components: vec![SectionComponent::RichText(DdRichText {
                    parent_class: None,
                    sal: crate::model::SalAnimation::Fade,
                    parent_copy: "This page does not exist.".to_string(),
                })],
            }],
        })],
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Site;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir(prefix: &str) -> std::path::PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{n}"))
    }

    #[test]
    fn export_writes_html_sitemap_robots_and_404() {
        let out = tmp_dir("dd_export_full");
        let site = Site::starter();
        let report = export_site(&site, &out, None).expect("export");
        assert_eq!(report.pages, 1);
        assert!(report.wrote_404);
        assert!(out.join("index.html").exists());
        assert!(out.join("sitemap.xml").exists());
        assert!(out.join("robots.txt").exists());
        assert!(out.join("404.html").exists());
        let html = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(html.contains("assets/css/style.min.css"));
        assert!(html.contains("lang=\"en\""));
        assert!(
            !out.join("assets/css/style.min.css").exists(),
            "css comes from grunt, not a bundled pack"
        );
        std::fs::remove_dir_all(&out).ok();
    }

    fn crate_source() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("source")
    }

    #[test]
    fn export_copies_source_webfonts_and_favicon_when_present() {
        let root = tmp_dir("dd_export_fonts_root");
        copy_dir_recursive(&crate_source().join("webfonts"), &root.join("source/webfonts"))
            .unwrap();
        copy_dir_recursive(&crate_source().join("favicon"), &root.join("source/favicon")).unwrap();
        let out = root.join("out");
        export_site(&Site::starter(), &out, Some(&root)).expect("export");
        assert!(out.join("assets/webfonts/fa-regular-400.woff2").exists());
        assert!(out.join("assets/favicon/favicon.png").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn export_copies_grunt_assets_when_dest_is_not_web() {
        let root = tmp_dir("dd_export_grunt_root");
        let grunt_css = root.join("web/assets/css");
        fs::create_dir_all(&grunt_css).unwrap();
        fs::write(grunt_css.join("style.min.css"), b"/* grunt */").unwrap();
        let out = root.join("dist");
        export_site(&Site::starter(), &out, Some(&root)).expect("export");
        let css = fs::read_to_string(out.join("assets/css/style.min.css")).unwrap();
        assert_eq!(css, "/* grunt */");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn export_to_web_does_not_clobber_grunt_css() {
        let root = tmp_dir("dd_export_noclobber");
        let grunt_css = root.join("web/assets/css");
        fs::create_dir_all(&grunt_css).unwrap();
        fs::write(grunt_css.join("style.min.css"), b"/* keep */").unwrap();
        export_site(&Site::starter(), &root.join("web"), Some(&root)).expect("export");
        let css = fs::read_to_string(root.join("web/assets/css/style.min.css")).unwrap();
        assert_eq!(css, "/* keep */");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn export_copies_source_images() {
        let root = tmp_dir("dd_export_imgs_root");
        let imgs = root.join("source").join("images");
        fs::create_dir_all(&imgs).unwrap();
        fs::write(imgs.join("hero.jpg"), b"fake").unwrap();
        let out = root.join("web");
        export_site(&Site::starter(), &out, Some(&root)).expect("export");
        assert!(out.join("assets/images/hero.jpg").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn sitemap_uses_base_url_and_skips_noindex() {
        let out = tmp_dir("dd_export_sitemap");
        let mut site = Site::starter();
        site.base_url = Some("https://ex.com".to_string());
        site.pages[0].head.robots = RobotsDirective::IndexFollow;
        site.pages.push(Page::from_template(
            "Hidden",
            crate::model::PageTemplate::HeroPlusSection,
        ));
        site.pages[1].head.robots = RobotsDirective::NoindexNofollow;
        export_site(&site, &out, None).expect("export");
        let map = fs::read_to_string(out.join("sitemap.xml")).unwrap();
        assert!(map.contains("https://ex.com/index.html"));
        assert!(!map.contains("hidden.html"));
        let robots = fs::read_to_string(out.join("robots.txt")).unwrap();
        assert!(robots.contains("Sitemap: https://ex.com/sitemap.xml"));
        std::fs::remove_dir_all(&out).ok();
    }

    #[test]
    fn does_not_overwrite_author_404_page() {
        let out = tmp_dir("dd_export_custom_404");
        let mut site = Site::starter();
        let mut p = Page::from_template("Not Found", crate::model::PageTemplate::HeroPlusSection);
        p.slug = "404".to_string();
        p.head.title = "Custom 404".to_string();
        site.pages.push(p);
        let report = export_site(&site, &out, None).expect("export");
        assert!(!report.wrote_404);
        let html = fs::read_to_string(out.join("404.html")).unwrap();
        assert!(html.contains("Custom 404"));
        std::fs::remove_dir_all(&out).ok();
    }
}
