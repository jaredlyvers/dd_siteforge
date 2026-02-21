use std::fs;
use std::path::Path;

use anyhow::Context;
use handlebars::Handlebars;
use serde_json::{Value, json};

use crate::model::{
    DdAccordion, DdAlert, DdBanner, DdCard, DdCta, DdHero, DdModal, DdSection, DdSlider, DdTabs,
    DdTimeline, Page, PageNode, SectionColumn, SectionComponent, Site,
};

const PAGE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{{title}}</title>
  {{#if meta_description}}<meta name="description" content="{{meta_description}}">{{/if}}
  <link rel="stylesheet" href="/assets/css/style.min.css">
</head>
<body class="dd-g">
<main>
{{{content}}}
</main>
<script src="/assets/js/main.min.js"></script>
</body>
</html>
"#;

pub fn render_site_to_dir(site: &Site, output_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir).context("failed to create export directory")?;
    for page in &site.pages {
        let html = render_page_html(page)?;
        let file_name = if page.slug == "index" {
            "index.html".to_string()
        } else {
            format!("{}.html", page.slug)
        };
        let out_path = output_dir.join(file_name);
        fs::write(&out_path, html)
            .with_context(|| format!("failed to write page output '{}'", out_path.display()))?;
    }
    Ok(())
}

pub fn render_page_html(page: &Page) -> anyhow::Result<String> {
    let mut hbs = Handlebars::new();
    hbs.register_template_string("page", PAGE_TEMPLATE)
        .context("failed to register page template")?;

    let mut content = String::new();
    for node in &page.nodes {
        match node {
            PageNode::Hero(hero) => content.push_str(&render_hero(hero)?),
            PageNode::Section(section) => content.push_str(&render_section(section)?),
        }
        content.push('\n');
    }

    hbs.render(
        "page",
        &json!({
            "title": page.title,
            "meta_description": page.meta_description,
            "content": content
        }),
    )
    .context("failed to render page template")
}

fn render_hero(hero: &DdHero) -> anyhow::Result<String> {
    let template = r#"<section class="dd-hero" aria-label="Hero section">
  <div class="dd-hero__image">
    <picture>
      {{#if image_mobile}}<source media="(max-width: 767px)" srcset="{{image_mobile}}">{{/if}}
      {{#if image_tablet}}<source media="(max-width: 1199px)" srcset="{{image_tablet}}">{{/if}}
      {{#if image_desktop}}<source media="(min-width: 1200px)" srcset="{{image_desktop}}">{{/if}}
      <img src="{{image}}" alt="{{image_alt}}" class="dd-img">
    </picture>
  </div>
  <div class="dd-hero__content dd-g" data-aos="fade-in">
    <div class="dd-hero__copy dd-u-1-1 dd-u-lg-12-24">
      <div class="dd-hero__title"><h1>{{title}}</h1></div>
      <div class="dd-hero__subtitle"><strong>{{subtitle}}</strong></div>
      {{#if copy}}<div class="dd-hero__body"><p>{{copy}}</p></div>{{/if}}
      {{#if cta_text}}<div class="dd-hero__cta"><a href="{{cta_link}}" target="{{cta_target}}" class="dd-button -primary">{{cta_text}}</a></div>{{/if}}
    </div>
  </div>
</section>"#;
    render_inline(template, hero_to_json(hero))
}

fn render_section(section: &DdSection) -> anyhow::Result<String> {
    let mut columns_html = String::new();
    for column in section_columns(section) {
        let mut inner = String::new();
        for component in &column.components {
            let html = match component {
                SectionComponent::Card(v) => render_card(v)?,
                SectionComponent::Alert(v) => render_alert(v)?,
                SectionComponent::Banner(v) => render_banner(v)?,
                SectionComponent::Tabs(v) => render_tabs(v)?,
                SectionComponent::Accordion(v) => render_accordion(v)?,
                SectionComponent::Cta(v) => render_cta(v)?,
                SectionComponent::Modal(v) => render_modal(v)?,
                SectionComponent::Slider(v) => render_slider(v)?,
                SectionComponent::Spacer(v) => render_inline(
                    r#"<div class="dd-spacer dd-spacer--{{height}}" aria-hidden="true"></div>"#,
                    serde_json::to_value(v)?,
                )?,
                SectionComponent::Timeline(v) => render_timeline(v)?,
            };
            inner.push_str(&html);
            inner.push('\n');
        }
        columns_html.push_str(&format!(
            r#"<div class="dd-section__item {}">{}</div>"#,
            column.width_class, inner
        ));
        columns_html.push('\n');
    }

    let template = r#"<section class="dd-section {{background}}" aria-label="Content section">
  <div class="dd-section__container dd-g">
    <div class="{{align}} {{width}} {{spacing}} dd-u-1-1 dd-g">
      {{{content}}}
    </div>
  </div>
</section>"#;

    render_inline(
        template,
        json!({
            "background": stringify_json(&serde_json::to_value(&section.background)?),
            "spacing": stringify_json(&serde_json::to_value(&section.spacing)?),
            "width": stringify_json(&serde_json::to_value(&section.width)?),
            "align": stringify_json(&serde_json::to_value(&section.align)?),
            "content": columns_html
        }),
    )
}

fn section_columns(section: &DdSection) -> Vec<SectionColumn> {
    if !section.columns.is_empty() {
        section.columns.clone()
    } else {
        vec![SectionColumn {
            id: format!("{}-legacy-column", section.id),
            width_class: "dd-u-1-1".to_string(),
            components: section.components.clone(),
        }]
    }
}

fn render_card(card: &DdCard) -> anyhow::Result<String> {
    let template = r#"<div class="dd-card">
  <div class="dd-card__items dd-g">
    <div class="dd-card__item l-box dd-u-1-1" data-aos="{{animate}}">
      <div class="dd-card__body dd-g">
        <div class="dd-card__image"><img src="{{image}}" alt="{{image_alt}}" class="dd-image" loading="lazy"></div>
        <div class="dd-card__copy l-box">
          <div class="dd-card__title"><h3>{{title}}</h3></div>
          {{#if subtitle}}<div class="dd-card__sub-title"><strong>{{subtitle}}</strong></div>{{/if}}
          {{#if copy}}<p>{{copy}}</p>{{/if}}
          {{#if cta_text}}<div class="dd-card__links"><a href="{{cta_link}}" class="dd-button -primary">{{cta_text}}</a></div>{{/if}}
        </div>
      </div>
    </div>
  </div>
</div>"#;
    let mut v = serde_json::to_value(card)?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "animate".to_string(),
            Value::String(
                card.animate
                    .as_ref()
                    .and_then(|a| serde_json::to_value(a).ok())
                    .map(|a| stringify_json(&a))
                    .unwrap_or_else(|| "fade-up".to_string()),
            ),
        );
    }
    render_inline(template, v)
}

fn render_alert(alert: &DdAlert) -> anyhow::Result<String> {
    let template = r#"<div class="dd-alert dd-alert--{{type}}">
  {{#if title}}<strong>{{title}}</strong>{{/if}}
  <p>{{message}}</p>
</div>"#;
    render_inline(template, serde_json::to_value(alert)?)
}

fn render_banner(banner: &DdBanner) -> anyhow::Result<String> {
    let template = r#"<div class="dd-banner" style="background: {{background}};">
  <p>{{message}}</p>
  {{#if link_text}}<a href="{{link_url}}">{{link_text}}</a>{{/if}}
</div>"#;
    render_inline(template, serde_json::to_value(banner)?)
}

fn render_tabs(tabs: &DdTabs) -> anyhow::Result<String> {
    let template = r#"<div class="dd-tabs dd-tabs--{{orientation}}">
  {{#each tabs}}
  <article class="dd-tabs__panel">
    <h3>{{title}}</h3>
    <div>{{content}}</div>
  </article>
  {{/each}}
</div>"#;
    let mut v = serde_json::to_value(tabs)?;
    if let Some(obj) = v.as_object_mut() {
        let orientation = tabs
            .orientation
            .as_ref()
            .and_then(|o| serde_json::to_value(o).ok())
            .map(|o| stringify_json(&o))
            .unwrap_or_else(|| "horizontal".to_string());
        obj.insert("orientation".to_string(), Value::String(orientation));
    }
    render_inline(template, v)
}

fn render_accordion(accordion: &DdAccordion) -> anyhow::Result<String> {
    let template = r#"<div class="dd-accordion">
  {{#each items}}
  <details>
    <summary>{{title}}</summary>
    <div>{{content}}</div>
  </details>
  {{/each}}
</div>"#;
    render_inline(template, serde_json::to_value(accordion)?)
}

fn render_cta(cta: &DdCta) -> anyhow::Result<String> {
    let template = r#"<section class="dd-cta">
  <h2>{{title}}</h2>
  <p>{{copy}}</p>
  <a href="{{cta_link}}" class="dd-button -primary">{{cta_text}}</a>
</section>"#;
    render_inline(template, serde_json::to_value(cta)?)
}

fn render_modal(modal: &DdModal) -> anyhow::Result<String> {
    let template = r#"<div class="dd-modal">
  <button class="dd-button -secondary">{{trigger_text}}</button>
  <div class="dd-modal__content" hidden>
    <h3>{{title}}</h3>
    <div>{{content}}</div>
  </div>
</div>"#;
    render_inline(template, serde_json::to_value(modal)?)
}

fn render_slider(slider: &DdSlider) -> anyhow::Result<String> {
    let template = r#"<div class="dd-slider" data-autoplay="{{autoplay}}" data-speed="{{speed}}">
  {{#each slides}}
  <article class="dd-slider__slide">
    <img src="{{image}}" alt="{{title}}" class="dd-image" loading="lazy">
    <h3>{{title}}</h3>
    <p>{{copy}}</p>
  </article>
  {{/each}}
</div>"#;
    let mut v = serde_json::to_value(slider)?;
    if let Some(obj) = v.as_object_mut() {
        obj.entry("autoplay".to_string())
            .or_insert(Value::Bool(false));
        obj.entry("speed".to_string()).or_insert(Value::from(400));
    }
    render_inline(template, v)
}

fn render_timeline(timeline: &DdTimeline) -> anyhow::Result<String> {
    let template = r#"<section class="dd-timeline">
  {{#each events}}
  <article class="dd-timeline__event">
    <time>{{date}}</time>
    <h3>{{title}}</h3>
    <p>{{description}}</p>
  </article>
  {{/each}}
</section>"#;
    render_inline(template, serde_json::to_value(timeline)?)
}

fn render_inline(template: &str, data: Value) -> anyhow::Result<String> {
    let mut hbs = Handlebars::new();
    hbs.register_template_string("inline", template)
        .context("failed to register inline template")?;
    hbs.render("inline", &data)
        .context("failed to render inline template")
}

fn hero_to_json(hero: &DdHero) -> Value {
    let cta_target = hero
        .cta_target
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "_self".to_string());

    json!({
        "image": hero.image,
        "title": hero.title,
        "subtitle": hero.subtitle,
        "copy": hero.copy,
        "cta_text": hero.cta_text,
        "cta_link": hero.cta_link,
        "cta_target": cta_target,
        "image_alt": hero.image_alt.clone().unwrap_or_default(),
        "image_mobile": hero.image_mobile,
        "image_tablet": hero.image_tablet,
        "image_desktop": hero.image_desktop
    })
}

fn stringify_json(value: &Value) -> String {
    match value {
        Value::String(v) => v.clone(),
        _ => String::new(),
    }
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
    }
}
