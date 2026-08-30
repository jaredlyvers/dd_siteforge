use std::fs;
use std::path::Path;

use anyhow::Context;
use serde_json::{json, Value};

use crate::templates::Renderer;
use crate::model::{
    DdAccordion, DdAlert, DdAlternating, DdBanner, DdBlockquote, DdCard, DdCta, DdFilmstrip,
    DdFooter, DdHead, DdHeader, DdHero, DdMilestones, DdModal, DdSection, DdSlider, Page,
    PageNode, SectionComponent, Site,
};

pub fn render_site_to_dir(site: &Site, output_dir: &Path, site_root: Option<&Path>) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir).context("failed to create export directory")?;
    let r = Renderer::load(site_root)?;
    let header_html = render_header(&r, &site.header)?;
    let footer_html = render_footer(&r, &site.footer)?;
    for page in &site.pages {
        let html = render_page_html_with_chrome(&r, page, &header_html, &footer_html, site)?;
        let file_name = crate::model::page_file_name(&page.slug);
        let out_path = output_dir.join(file_name);
        fs::write(&out_path, html)
            .with_context(|| format!("failed to write page output '{}'", out_path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
fn render_page_html(page: &Page) -> anyhow::Result<String> {
    let site = Site::starter();
    let r = Renderer::bundled_only()?;
    render_page_html_with_chrome(&r, page, "", "", &site)
}

pub fn render_page_html_with_chrome(
    r: &Renderer,
    page: &Page,
    header_html: &str,
    footer_html: &str,
    site: &Site,
) -> anyhow::Result<String> {

    let mut content = String::new();
    for node in &page.nodes {
        match node {
            PageNode::Hero(hero) => content.push_str(&render_hero(r, hero)?),
            PageNode::Section(section) => content.push_str(&render_section(r, section)?),
        }
        content.push('\n');
    }

    let head_html = render_head(r, &page.head, site, page)?;
    let lang = if site.lang.trim().is_empty() {
        "en"
    } else {
        site.lang.trim()
    };

    r.render(
        "_page",
        &json!({
            "lang": lang,
            "head_html": head_html,
            "header_html": header_html,
            "footer_html": footer_html,
            "content": content
        }),
    )
}

fn render_head(r: &Renderer, head: &DdHead, site: &Site, page: &Page) -> anyhow::Result<String> {
    let robots = robots_token(head.robots);
    let schema_type = schema_type_token(head.schema_type);
    let mut schema = serde_json::Map::new();
    schema.insert(
        "@context".to_string(),
        Value::String("https://schema.org".to_string()),
    );
    schema.insert("@type".to_string(), Value::String(schema_type.to_string()));
    schema.insert("name".to_string(), Value::String(head.title.clone()));
    if let Some(d) = head
        .meta_description
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        schema.insert("description".to_string(), Value::String(d.to_string()));
    }
    let file = crate::model::page_href(&page.slug);
    let stored_canonical = head
        .canonical_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let canonical = stored_canonical
        .clone()
        .or_else(|| crate::model::absolute_url(site.base_url.as_deref(), &file));
    if let Some(u) = canonical.as_deref() {
        schema.insert("url".to_string(), Value::String(u.to_string()));
    }
    if let Some(i) = head
        .og_image
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        schema.insert("image".to_string(), Value::String(i.to_string()));
    }
    let schema_json = serde_json::to_string_pretty(&Value::Object(schema))
        .unwrap_or_else(|_| "{}".to_string());

    let og_title = head
        .og_title
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| Some(head.title.clone()));
    let og_description = head
        .og_description
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| {
            head.meta_description
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        });
    let og_image = head
        .og_image
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| public_url(v));
    let og_url = canonical.clone();
    let twitter_card = og_image
        .as_ref()
        .map(|_| "summary_large_image".to_string());

    let data = json!({
        "title": head.title,
        "meta_description": head.meta_description,
        "canonical_url": canonical,
        "robots": robots,
        "og_title": og_title,
        "og_description": og_description,
        "og_image": og_image,
        "og_url": og_url,
        "twitter_card": twitter_card,
        "schema_json": schema_json,
    });
    r.render("_head", &data)
}

pub(crate) fn render_header(r: &Renderer, header: &DdHeader) -> anyhow::Result<String> {
    let custom = header
        .custom_css
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| format!(" {}", v))
        .unwrap_or_default();
    let alert_html = if let Some(alert) = &header.alert {
        render_alert(r, alert)?
    } else {
        String::new()
    };
    let mut sections_html = String::new();
    for section in &header.sections {
        sections_html.push_str(&render_section(r, section)?);
        sections_html.push('\n');
    }
    r.render(
        "dd-header",
        &json!({
            "custom": custom,
            "alert_html": alert_html,
            "sections_html": sections_html,
        }),
    )
}

pub(crate) fn render_footer(r: &Renderer, footer: &DdFooter) -> anyhow::Result<String> {
    let custom = footer
        .custom_css
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| format!(" {}", v))
        .unwrap_or_default();
    let mut sections_html = String::new();
    for section in &footer.sections {
        sections_html.push_str(&render_section(r, section)?);
        sections_html.push('\n');
    }
    r.render(
        "dd-footer",
        &json!({
            "custom": custom,
            "sections_html": sections_html,
        }),
    )
}

fn robots_token(r: crate::model::RobotsDirective) -> &'static str {
    match r {
        crate::model::RobotsDirective::IndexFollow => "index, follow",
        crate::model::RobotsDirective::NoindexFollow => "noindex, follow",
        crate::model::RobotsDirective::IndexNofollow => "index, nofollow",
        crate::model::RobotsDirective::NoindexNofollow => "noindex, nofollow",
    }
}

fn schema_type_token(s: crate::model::SchemaType) -> &'static str {
    match s {
        crate::model::SchemaType::WebPage => "WebPage",
        crate::model::SchemaType::Article => "Article",
        crate::model::SchemaType::AboutPage => "AboutPage",
        crate::model::SchemaType::ContactPage => "ContactPage",
        crate::model::SchemaType::CollectionPage => "CollectionPage",
        crate::model::SchemaType::Organization => "Organization",
        crate::model::SchemaType::LocalBusiness => "LocalBusiness",
        crate::model::SchemaType::Product => "Product",
        crate::model::SchemaType::Service => "Service",
    }
}

fn render_hero(r: &Renderer, hero: &DdHero) -> anyhow::Result<String> {
    r.render("dd-hero", &hero_to_json(hero))
}

fn render_section(r: &Renderer, section: &DdSection) -> anyhow::Result<String> {
    let mut columns_html = String::new();
    let item_box_class = section
        .item_box_class
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "l-box".to_string());
    for column in &section.columns {
        let mut inner = String::new();
        for component in &column.components {
            let html = match component {
                SectionComponent::Alternating(v) => render_alternating(r, v)?,
                SectionComponent::Card(v) => render_card(r, v)?,
                SectionComponent::Cta(v) => render_cta(r, v)?,
                SectionComponent::Filmstrip(v) => render_filmstrip(r, v)?,
                SectionComponent::Milestones(v) => render_milestones(r, v)?,
                SectionComponent::Slider(v) => render_slider(r, v)?,
                SectionComponent::Modal(v) => render_modal(r, v)?,
                SectionComponent::Banner(v) => render_banner(r, v)?,
                SectionComponent::Accordion(v) => render_accordion(r, v)?,
                SectionComponent::Blockquote(v) => render_blockquote(r, v)?,
                SectionComponent::Alert(v) => render_alert(r, v)?,
                SectionComponent::Image(v) => render_image(r, v)?,
                SectionComponent::RichText(v) => render_rich_text(r, v)?,
                SectionComponent::Navigation(v) => render_navigation(r, v)?,
                SectionComponent::HeaderSearch(v) => render_header_search(r, v)?,
                SectionComponent::HeaderMenu(v) => render_header_menu(r, v)?,
            };
            inner.push_str(&html);
            inner.push('\n');
        }
        columns_html.push_str(&r.render(
            "dd-section-column",
            &json!({
                "width_class": column.width_class,
                "item_box_class": item_box_class,
                "inner": inner,
            }),
        )?);
        columns_html.push('\n');
    }


    r.render(
        "dd-section",
        &json!({
            "section_class": section
                .section_class
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok())
                .map(|v| stringify_json(&v))
                .unwrap_or_else(|| "-full-contained".to_string()),
            "section_title": section.section_title,
            "content": columns_html
        }),
    )
}

fn render_alternating(r: &Renderer, alternating: &DdAlternating) -> anyhow::Result<String> {
    let mut v = serde_json::to_value(alternating)?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "parent_type".to_string(),
            Value::String(
                serde_json::to_value(alternating.parent_type)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "-default".to_string()),
            ),
        );
        obj.insert(
            "sal".to_string(),
            Value::String(
                serde_json::to_value(alternating.sal)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade".to_string()),
            ),
        );
        if let Some(items) = obj.get_mut("items").and_then(|v| v.as_array_mut()) {
            attach_sal_stagger(items);
        }
    }
    r.render("dd-alternating", &v)
}

fn render_card(r: &Renderer, card: &DdCard) -> anyhow::Result<String> {
    let mut items = Vec::new();
    for item in &card.items {
        let link_url = item
            .child_link_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let link_label = item
            .child_link_label
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let has_link = link_url.is_some() && link_label.is_some();
        let link_target = item
            .child_link_target
            .as_ref()
            .and_then(|v| serde_json::to_value(v).ok())
            .map(|v| stringify_json(&v))
            .unwrap_or_else(|| "_self".to_string());
        items.push(json!({
            "child_image_url": item.child_image_url,
            "child_image_alt": item.child_image_alt,
            "child_title": item.child_title,
            "child_subtitle": item.child_subtitle,
            "child_copy": item.child_copy,
            "child_link_url": link_url.unwrap_or_default(),
            "child_link_target": link_target,
            "child_link_label": link_label.unwrap_or_default(),
            "has_link": has_link
        }));
    }
    attach_sal_stagger(&mut items);
    let data = json!({
        "parent_type": serde_json::to_value(card.parent_type).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "sal": serde_json::to_value(card.sal).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade".to_string()),
        "parent_width": card.parent_width,
        "items": items
    });
    r.render("dd-card", &data)
}

fn render_banner(r: &Renderer, banner: &DdBanner) -> anyhow::Result<String> {
    let mut v = serde_json::to_value(banner)?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "parent_class".to_string(),
            Value::String(
                serde_json::to_value(banner.parent_class)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "-bg-center-center".to_string()),
            ),
        );
        obj.insert(
            "sal".to_string(),
            Value::String(
                serde_json::to_value(banner.sal)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade".to_string()),
            ),
        );
    }
    r.render("dd-banner", &v)
}

fn render_cta(r: &Renderer, cta: &DdCta) -> anyhow::Result<String> {

    let link_url = cta
        .parent_link_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let link_label = cta
        .parent_link_label
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let has_link = link_url.is_some() && link_label.is_some();
    let link_target = cta
        .parent_link_target
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "_self".to_string());

    let data = json!({
        "parent_class": serde_json::to_value(cta.parent_class).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-top-left".to_string()),
        "parent_image_url": cta.parent_image_url,
        "parent_image_alt": cta.parent_image_alt,
        "sal": serde_json::to_value(cta.sal).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade".to_string()),
        "parent_title": cta.parent_title,
        "parent_subtitle": cta.parent_subtitle,
        "parent_copy": cta.parent_copy,
        "parent_link_url": link_url.unwrap_or_default(),
        "parent_link_target": link_target,
        "parent_link_label": link_label.unwrap_or_default(),
        "has_link": has_link
    });
    r.render("dd-cta", &data)
}

fn render_filmstrip(r: &Renderer, filmstrip: &DdFilmstrip) -> anyhow::Result<String> {

    let data = json!({
        "parent_type": serde_json::to_value(filmstrip.parent_type).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "sal": serde_json::to_value(filmstrip.sal).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade".to_string()),
        "items": filmstrip.items
    });
    r.render("dd-filmstrip", &data)
}

fn render_milestones(r: &Renderer, milestones: &DdMilestones) -> anyhow::Result<String> {
    let mut items = Vec::new();
    for item in &milestones.items {
        let link_url = item
            .child_link_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let link_label = item
            .child_link_label
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let has_link = link_url.is_some() && link_label.is_some();
        let link_target = item
            .child_link_target
            .as_ref()
            .and_then(|v| serde_json::to_value(v).ok())
            .map(|v| stringify_json(&v))
            .unwrap_or_else(|| "_self".to_string());
        items.push(json!({
            "child_percentage": item.child_percentage,
            "child_title": item.child_title,
            "child_subtitle": item.child_subtitle,
            "child_copy": item.child_copy,
            "child_link_url": link_url.unwrap_or_default(),
            "child_link_target": link_target,
            "child_link_label": link_label.unwrap_or_default(),
            "has_link": has_link
        }));
    }
    attach_sal_stagger(&mut items);
    let data = json!({
        "sal": serde_json::to_value(milestones.sal).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade".to_string()),
        "parent_width": milestones.parent_width,
        "items": items
    });
    r.render("dd-milestones", &data)
}

fn render_modal(r: &Renderer, modal: &DdModal) -> anyhow::Result<String> {
    let data = json!({
        "parent_title": modal.parent_title,
        "parent_copy": modal.parent_copy,
        "parent_modal_id": html_id_safe_from_title(&modal.parent_title, "modal")
    });
    r.render("dd-modal", &data)
}

fn render_slider(r: &Renderer, slider: &DdSlider) -> anyhow::Result<String> {

    let fallback_uid = stable_uid_from_title(&slider.parent_title);
    let parent_uid = html_id_safe_from_title(&slider.parent_title, &fallback_uid);
    let mut items = Vec::new();
    for item in &slider.items {
        let link_url = item
            .child_link_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let link_label = item
            .child_link_label
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let has_link = link_url.is_some() && link_label.is_some();
        let link_target = item
            .child_link_target
            .as_ref()
            .and_then(|v| serde_json::to_value(v).ok())
            .map(|v| stringify_json(&v))
            .unwrap_or_else(|| "_self".to_string());
        items.push(json!({
            "child_title": item.child_title,
            "child_copy": item.child_copy,
            "child_link_url": link_url.unwrap_or_default(),
            "child_link_target": link_target,
            "child_link_label": link_label.unwrap_or_default(),
            "child_image_url": item.child_image_url,
            "child_image_alt": item.child_image_alt,
            "has_link": has_link
        }));
    }
    let data = json!({
        "parent_title": slider.parent_title,
        "has_parent_title": !slider.parent_title.trim().is_empty(),
        "parent_uid": parent_uid,
        "items": items
    });
    r.render("dd-slider", &data)
}

fn render_accordion(r: &Renderer, accordion: &DdAccordion) -> anyhow::Result<String> {
    let mut v = serde_json::to_value(accordion)?;
    let faq_schema = serde_json::to_string(&json!({
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": accordion.items.iter().map(|item| {
            json!({
                "@type": "Question",
                "name": item.child_title,
                "acceptedAnswer": {
                    "@type": "Answer",
                    "text": item.child_copy
                }
            })
        }).collect::<Vec<_>>()
    }))?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "parent_type".to_string(),
            Value::String(
                serde_json::to_value(accordion.parent_type)
                    .map(|v| stringify_json(&v))
                    .unwrap_or_else(|_| "-default".to_string()),
            ),
        );
        obj.insert(
            "parent_class".to_string(),
            Value::String(
                serde_json::to_value(accordion.parent_class)
                    .map(|v| stringify_json(&v))
                    .unwrap_or_else(|_| "-primary".to_string()),
            ),
        );
        obj.insert(
            "sal".to_string(),
            Value::String(
                serde_json::to_value(accordion.sal)
                    .map(|v| stringify_json(&v))
                    .unwrap_or_else(|_| "fade".to_string()),
            ),
        );
        obj.insert(
            "has_faq_schema".to_string(),
            Value::Bool(matches!(
                accordion.parent_type,
                crate::model::AccordionType::Faq
            )),
        );
        obj.insert("faq_schema_json".to_string(), Value::String(faq_schema));
    }
    r.render("dd-accordion", &v)
}

fn render_blockquote(r: &Renderer, blockquote: &DdBlockquote) -> anyhow::Result<String> {
    let blockquote_schema_json = serde_json::to_string(&json!({
      "@context": "https://schema.org/",
      "@type": "Quotation",
      "creator": {
        "@type": "Person",
        "name": format!(
            "{}, {}",
            blockquote.parent_name, blockquote.parent_role
        )
      },
      "text": blockquote.parent_copy
    }))?;
    let mut v = serde_json::to_value(blockquote)?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "sal".to_string(),
            Value::String(
                serde_json::to_value(blockquote.sal)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade".to_string()),
            ),
        );
        obj.insert(
            "blockquote_schema_json".to_string(),
            Value::String(blockquote_schema_json),
        );
    }
    r.render("dd-blockquote", &v)
}

fn render_alert(r: &Renderer, alert: &DdAlert) -> anyhow::Result<String> {
    let data = json!({
        "parent_type": serde_json::to_value(alert.parent_type).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "parent_class": serde_json::to_value(alert.parent_class).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "sal": serde_json::to_value(alert.sal).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade".to_string()),
        "parent_title": alert.parent_title.as_deref().unwrap_or(""),
        "has_title": alert.parent_title.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false),
        "parent_copy": alert.parent_copy
    });
    r.render("dd-alert", &data)
}

fn render_image(r: &Renderer, image: &crate::model::DdImage) -> anyhow::Result<String> {
    let data_aos = sal_token(image.sal);
    let has_link = image
        .parent_link_url
        .as_deref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let link_target = image
        .parent_link_target
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "_self".to_string());
    let data = json!({
        "sal": data_aos,
        "has_link": has_link,
        "parent_image_url": image.parent_image_url,
        "parent_image_alt": image.parent_image_alt,
        "parent_link_url": image.parent_link_url.clone().unwrap_or_default(),
        "parent_link_target": link_target,
    });
    r.render("dd-image", &data)
}

fn render_rich_text(r: &Renderer, rt: &crate::model::DdRichText) -> anyhow::Result<String> {
    let parent_class = rt
        .parent_class
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let parent_copy_html = markdown_to_html(&rt.parent_copy);
    let data = json!({
        "parent_class": parent_class,
        "sal": sal_token(rt.sal),
        "parent_copy_html": parent_copy_html,
    });
    r.render("dd-rich_text", &data)
}

fn render_navigation(r: &Renderer, nav: &crate::model::DdNavigation) -> anyhow::Result<String> {
    let parent_class = navigation_class_token(nav.parent_class);
    let aria_label = match nav.parent_type {
        crate::model::NavigationType::HeaderNav => "header navigation",
        crate::model::NavigationType::FooterNav => "footer navigation",
    };
    let items_html = render_nav_items(&nav.items);
    r.render(
        "dd-navigation",
        &json!({
            "parent_class": parent_class,
            "sal": sal_token(nav.sal),
            "aria_label": aria_label,
            "items_html": items_html,
        }),
    )
}

fn render_nav_items(items: &[crate::model::NavigationItem]) -> String {
    let mut out = String::new();
    for item in items {
        out.push_str(&render_nav_item(item));
        out.push('\n');
    }
    out
}

fn render_nav_item(item: &crate::model::NavigationItem) -> String {
    let has_children = !item.items.is_empty();
    let has_children_cls = if has_children { " -has-children" } else { "" };
    let css = item.child_link_css.as_deref().unwrap_or("");
    let label = item.child_link_label.as_str();
    let inner = match item.child_kind {
        crate::model::NavigationKind::Link => {
            let url = item.child_link_url.as_deref().unwrap_or("");
            let target = item
                .child_link_target
                .map(link_target_token)
                .unwrap_or("_self");
            format!(
                r#"<a href="{url}" target="{target}" class="{css}">{label}</a>"#,
                url = url,
                target = target,
                css = css,
                label = label,
            )
        }
        crate::model::NavigationKind::Button => {
            format!(
                r#"<span class="{css}" role="presentation">{label}</span>"#,
                css = css,
                label = label,
            )
        }
    };
    let submenu = if has_children {
        format!(
            r#"
        <ul class="sub-menu">
{children}
        </ul>"#,
            children = render_nav_items(&item.items),
        )
    } else {
        String::new()
    };
    format!(
        r#"      <li class="menu-item{has_children_cls}">
        {inner}{submenu}
      </li>"#,
        has_children_cls = has_children_cls,
        inner = inner,
        submenu = submenu,
    )
}

fn render_header_search(r: &Renderer, search: &crate::model::DdHeaderSearch) -> anyhow::Result<String> {
    let data = json!({
        "parent_width": search.parent_width,
        "sal": sal_token(search.sal),
    });
    r.render("dd-header-search", &data)
}

fn render_header_menu(r: &Renderer, menu: &crate::model::DdHeaderMenu) -> anyhow::Result<String> {
    let data = json!({
        "parent_width": menu.parent_width,
        "sal": sal_token(menu.sal),
    });
    r.render("dd-header-menu", &data)
}

fn sal_token(sal: crate::model::SalAnimation) -> String {
    serde_json::to_value(sal)
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|_| "fade".to_string())
}

fn attach_sal_stagger(items: &mut [Value]) {
    for (i, item) in items.iter_mut().enumerate() {
        let Some(obj) = item.as_object_mut() else {
            continue;
        };
        let delay = (i as u32 * 100).min(1000);
        if delay > 0 {
            obj.insert("sal_delay".to_string(), json!(delay));
        }
    }
}

fn link_target_token(target: crate::model::CardLinkTarget) -> &'static str {
    match target {
        crate::model::CardLinkTarget::SelfTarget => "_self",
        crate::model::CardLinkTarget::Blank => "_blank",
    }
}

fn navigation_class_token(class: crate::model::NavigationClass) -> &'static str {
    match class {
        crate::model::NavigationClass::MainMenu => "-main-menu",
        crate::model::NavigationClass::MenuSecondary => "-menu-secondary",
        crate::model::NavigationClass::MenuTertiary => "-menu-tertiary",
        crate::model::NavigationClass::FooterMenu => "-footer-menu",
        crate::model::NavigationClass::FooterMenuSecondary => "-footer-menu-secondary",
        crate::model::NavigationClass::FooterMenuTertiary => "-footer-menu-tertiary",
        crate::model::NavigationClass::SocialMenu => "-social-menu",
    }
}


fn hero_to_json(hero: &DdHero) -> Value {
    let link_1_target = hero
        .link_1_target
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "_self".to_string());
    let link_2_target = hero
        .link_2_target
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "_self".to_string());
    let image = hero.parent_image_url.trim();
    let subtitle = hero.parent_subtitle.trim();
    let parent_class = hero
        .parent_class
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v));
    let sal = hero
        .sal
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "fade".to_string());
    let parent_custom_css = hero
        .parent_custom_css
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let parent_copy_html = hero
        .parent_copy
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .map(markdown_to_html);
    let has_link_1 = hero
        .link_1_label
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
        && hero
            .link_1_url
            .as_deref()
            .is_some_and(|v| !v.trim().is_empty());
    let has_link_2 = hero
        .link_2_label
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
        && hero
            .link_2_url
            .as_deref()
            .is_some_and(|v| !v.trim().is_empty());
    let bg_mobile = hero
        .parent_image_mobile
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(image);
    let bg_desktop = hero
        .parent_image_desktop
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(image);
    let parent_image_class = hero
        .parent_image_class
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "-full-full".to_string());
    let has_image = !image.is_empty();
    let has_body = hero
        .parent_copy
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
        || has_link_1
        || has_link_2;

    json!({
        "parent_image_url": public_url(&hero.parent_image_url),
        "parent_class": parent_class,
        "sal": sal,
        "parent_custom_css": parent_custom_css,
        "parent_title": hero.parent_title,
        "parent_subtitle": if subtitle.is_empty() { None } else { Some(hero.parent_subtitle.clone()) },
        "parent_copy_html": parent_copy_html,
        "link_1_label": hero.link_1_label,
        "link_1_url": hero.link_1_url.as_deref().map(public_url),
        "link_1_target": link_1_target,
        "link_2_label": hero.link_2_label,
        "link_2_url": hero.link_2_url.as_deref().map(public_url),
        "link_2_target": link_2_target,
        "parent_image_alt": hero.parent_image_alt.clone().unwrap_or_default(),
        "parent_image_mobile": hero.parent_image_mobile.as_deref().map(public_url),
        "parent_image_tablet": hero.parent_image_tablet.as_deref().map(public_url),
        "parent_image_desktop": hero.parent_image_desktop.as_deref().map(public_url),
        "parent_image_class": parent_image_class,
        "has_image": has_image,
        "has_body": has_body,
        "has_links": has_link_1 || has_link_2,
        "has_link_1": has_link_1,
        "has_link_2": has_link_2,
        "bg_mobile": public_url(bg_mobile),
        "bg_desktop": public_url(bg_desktop)
    })
}

fn public_url(stored: &str) -> String {
    let t = stored.trim();
    if t.starts_with("http://")
        || t.starts_with("https://")
        || t.starts_with('#')
        || t.starts_with("mailto:")
        || t.starts_with("tel:")
    {
        t.to_string()
    } else {
        t.trim_start_matches('/').to_string()
    }
}

fn markdown_to_html(input: &str) -> String {
    let blocks = input.split("\n\n");
    let mut out = String::new();
    for block in blocks {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }
        if looks_like_html_block(trimmed) {
            out.push_str(trimmed);
            out.push('\n');
            continue;
        }
        let inline = inline_markdown_to_html(trimmed).replace('\n', "<br/>\n");
        out.push_str("<p>");
        out.push_str(&inline);
        out.push_str("</p>\n");
    }
    out
}

fn looks_like_html_block(input: &str) -> bool {
    input.starts_with('<') && input.ends_with('>')
}

fn inline_markdown_to_html(input: &str) -> String {
    let mut escaped = input.to_string();
    escaped = replace_md_link(&escaped);
    escaped = replace_md_wrapped(&escaped, "**", "<strong>", "</strong>");
    escaped = replace_md_wrapped(&escaped, "*", "<em>", "</em>");
    replace_md_wrapped(&escaped, "`", "<code>", "</code>")
}

fn replace_md_wrapped(input: &str, token: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    let mut open_state = false;
    while let Some(pos) = rest.find(token) {
        out.push_str(&rest[..pos]);
        out.push_str(if open_state { close } else { open });
        open_state = !open_state;
        rest = &rest[pos + token.len()..];
    }
    out.push_str(rest);
    out
}

fn replace_md_link(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    loop {
        let Some(lb) = rest.find('[') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..lb]);
        let after_lb = &rest[lb + 1..];
        let Some(rb) = after_lb.find(']') else {
            out.push_str(&rest[lb..]);
            break;
        };
        let link_text = &after_lb[..rb];
        let after_rb = &after_lb[rb + 1..];
        if !after_rb.starts_with('(') {
            out.push('[');
            rest = after_lb;
            continue;
        }
        let after_paren = &after_rb[1..];
        let Some(cp) = after_paren.find(')') else {
            out.push_str(&rest[lb..]);
            break;
        };
        let href = &after_paren[..cp];
        out.push_str("<a href=\"");
        out.push_str(href);
        out.push_str("\">");
        out.push_str(link_text);
        out.push_str("</a>");
        rest = &after_paren[cp + 1..];
    }
    out
}

fn stringify_json(value: &Value) -> String {
    match value {
        Value::String(v) => v.clone(),
        _ => String::new(),
    }
}

fn html_id_safe_from_title(title: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in title.trim().to_lowercase().chars() {
        let keep = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-';
        if keep {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let mut out = out.trim_matches('-').to_string();
    if out.is_empty() {
        out = fallback.to_string();
    }
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out = format!("modal-{out}");
    }
    out
}

fn stable_uid_from_title(title: &str) -> String {
    let mut hash: u64 = 5381;
    for b in title.as_bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(*b as u64);
    }
    format!("uid-{:06}", hash % 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::render_page_html;
    use crate::model::Site;

    #[test]
    fn renders_page_with_hero_and_section() {
        let site = Site::starter();
        let page = &site.pages[0];
        let html = render_page_html(page).expect("page should render");
        assert!(html.contains("dd-hero"));
        assert!(html.contains("dd-section"));
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("assets/css/style.min.css"));
        assert!(html.contains("lang=\"en\""));
        assert!(!html.contains("/assets/css/style.min.css"));
        assert!(html.contains("data-sal="));
        assert!(!html.contains("data-aos"));
    }

    #[test]
    fn card_items_stagger_sal_delay() {
        use crate::model::{CardItem, CardLinkTarget, CardType, DdCard, Page, PageNode, SalAnimation, SectionClass, SectionColumn, SectionComponent, SectionItemBoxClass};

        let card = DdCard {
            parent_type: CardType::Default,
            sal: SalAnimation::SlideUp,
            parent_width: "dd-u-1-1".to_string(),
            items: vec![
                CardItem {
                    child_image_url: "/a.jpg".to_string(),
                    child_image_alt: "a".to_string(),
                    child_title: "A".to_string(),
                    child_subtitle: String::new(),
                    child_copy: "one".to_string(),
                    child_link_url: None,
                    child_link_target: Some(CardLinkTarget::SelfTarget),
                    child_link_label: None,
                },
                CardItem {
                    child_image_url: "/b.jpg".to_string(),
                    child_image_alt: "b".to_string(),
                    child_title: "B".to_string(),
                    child_subtitle: String::new(),
                    child_copy: "two".to_string(),
                    child_link_url: None,
                    child_link_target: Some(CardLinkTarget::SelfTarget),
                    child_link_label: None,
                },
            ],
        };
        let page = Page {
            id: "p".to_string(),
            slug: "index".to_string(),
            slug_locked: false,
            head: crate::model::Site::starter().pages[0].head.clone(),
            nodes: vec![PageNode::Section(crate::model::DdSection {
                id: "s1".to_string(),
                section_title: None,
                section_class: Some(SectionClass::Contained),
                item_box_class: Some(SectionItemBoxClass::LBox),
                columns: vec![SectionColumn {
                    id: "c1".to_string(),
                    width_class: "dd-u-1-1".to_string(),
                    components: vec![SectionComponent::Card(card)],
                }],
            })],
        };
        let html = render_page_html(&page).expect("card page should render");
        assert!(html.contains("data-sal=\"slide-up\""));
        assert!(!html.contains("data-aos"));
        assert!(html.contains("data-sal-delay=\"100\""));
        let first = html.find("dd-card__item").expect("item");
        let delay = html.find("data-sal-delay=\"100\"").expect("stagger");
        assert!(delay > first, "delay should land on a later card item");
    }

    #[test]
    fn auto_canonical_and_og_url_from_base_url() {
        let mut site = Site::starter();
        site.base_url = Some("https://ex.com".to_string());
        site.lang = "fr".to_string();
        let r = crate::templates::Renderer::bundled_only().unwrap();
        let header = super::render_header(&r, &site.header).unwrap();
        let footer = super::render_footer(&r, &site.footer).unwrap();
        let html =
            super::render_page_html_with_chrome(&r, &site.pages[0], &header, &footer, &site).unwrap();
        assert!(html.contains("lang=\"fr\""));
        assert!(html.contains("rel=\"canonical\" href=\"https://ex.com/index.html\""));
        assert!(html.contains("property=\"og:url\" content=\"https://ex.com/index.html\""));
        assert!(html.contains("property=\"og:title\" content=\"Home\""));
    }
}
