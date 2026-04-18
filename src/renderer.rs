use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use handlebars::Handlebars;
use serde_json::{json, Value};

use crate::model::{
    DdAccordion, DdAlert, DdAlternating, DdBanner, DdBlockquote, DdCard, DdCta, DdFilmstrip,
    DdFooter, DdHead, DdHeader, DdHero, DdMilestones, DdModal, DdSection, DdSlider, Page,
    PageNode, SectionComponent, Site,
};

const PAGE_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
{{{head_html}}}
<body class="dd-g">
{{{header_html}}}
<main>
{{{content}}}
</main>
{{{footer_html}}}
<script src="/assets/js/main.min.js"></script>
</body>
</html>
"#;

pub fn render_site_to_dir(site: &Site, output_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(output_dir).context("failed to create export directory")?;
    let header_html = render_header(&site.header)?;
    let footer_html = render_footer(&site.footer)?;
    for page in &site.pages {
        let html = render_page_html_with_chrome(page, &header_html, &footer_html)?;
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
    // Render a single page without header/footer chrome — used by tests and callers
    // that don't have access to the full site (e.g. previews).
    render_page_html_with_chrome(page, "", "")
}

pub fn render_page_html_with_chrome(
    page: &Page,
    header_html: &str,
    footer_html: &str,
) -> anyhow::Result<String> {
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

    let head_html = render_head(&page.head)?;

    hbs.render(
        "page",
        &json!({
            "head_html": head_html,
            "header_html": header_html,
            "footer_html": footer_html,
            "content": content
        }),
    )
    .context("failed to render page template")
}

fn render_head(head: &DdHead) -> anyhow::Result<String> {
    let template = r##"<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{{title}}</title>
  {{#if meta_description}}<meta name="description" content="{{meta_description}}">{{/if}}
  {{#if canonical_url}}<link rel="canonical" href="{{canonical_url}}">{{/if}}
  <meta name="robots" content="{{robots}}">
  {{#if og_title}}<meta property="og:title" content="{{og_title}}">{{/if}}
  {{#if og_description}}<meta property="og:description" content="{{og_description}}">{{/if}}
  {{#if og_image}}<meta property="og:image" content="{{og_image}}">{{/if}}
  <link rel="apple-touch-icon" sizes="180x180" href="/assets/favicon/apple-touch-icon.png" />
  <link rel="icon" type="image/png" sizes="32x32" href="/assets/favicon/favicon-32x32.png" />
  <link rel="icon" type="image/png" sizes="16x16" href="/assets/favicon/favicon-16x16.png" />
  <link rel="manifest" href="/assets/favicon/site.webmanifest" />
  <link rel="mask-icon" href="/assets/favicon/safari-pinned-tab.svg" color="#5bbad5" />
  <link rel="shortcut icon" href="/assets/favicon/favicon.ico" />
  <meta name="msapplication-TileColor" content="#ffffff" />
  <meta name="msapplication-config" content="/assets/favicon/browserconfig.xml" />
  <meta name="theme-color" content="#ffffff" />
  <link rel="stylesheet" href="/assets/css/style.min.css">
  <script type="application/ld+json">{{{schema_json}}}</script>
</head>"##;

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
    if let Some(u) = head
        .canonical_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
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

    let data = json!({
        "title": head.title,
        "meta_description": head.meta_description,
        "canonical_url": head.canonical_url,
        "robots": robots,
        "og_title": head.og_title,
        "og_description": head.og_description,
        "og_image": head.og_image,
        "schema_json": schema_json,
    });
    render_inline(template, data)
}

fn render_header(header: &DdHeader) -> anyhow::Result<String> {
    let custom = header
        .custom_css
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| format!(" {}", v))
        .unwrap_or_default();
    let alert_html = if let Some(alert) = &header.alert {
        render_alert(alert)?
    } else {
        String::new()
    };
    let mut sections_html = String::new();
    for section in &header.sections {
        sections_html.push_str(&render_section(section)?);
        sections_html.push('\n');
    }
    Ok(format!(
        r#"<header class="dd-header{custom}">
{alert_html}
  <div class="dd-header__top">
{sections_html}
  </div>
  <div class="dd-search">
    <button class="dd-search__close">- search</button>
    <form action="">
      <label for="name">Search<br />
        <input type="text" id="name">
      </label>
    </form>
  </div>
</header>"#,
        custom = custom,
        alert_html = alert_html,
        sections_html = sections_html,
    ))
}

fn render_footer(footer: &DdFooter) -> anyhow::Result<String> {
    let custom = footer
        .custom_css
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| format!(" {}", v))
        .unwrap_or_default();
    let mut sections_html = String::new();
    for section in &footer.sections {
        sections_html.push_str(&render_section(section)?);
        sections_html.push('\n');
    }
    Ok(format!(
        r#"<footer class="dd-footer{custom}">
  <div class="dd-footer__content">
{sections_html}
  </div>
</footer>"#,
        custom = custom,
        sections_html = sections_html,
    ))
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

fn render_hero(hero: &DdHero) -> anyhow::Result<String> {
    let template = r#"<section class="dd-hero{{#if parent_class}} {{parent_class}}{{/if}}{{#if parent_custom_css}} {{parent_custom_css}}{{/if}}" aria-label="Introduction">
  {{#if has_image}}<div class="dd-hero__image {{parent_image_class}}">
    <picture>
      {{#if parent_image_mobile}}<source media="(max-width: 767px)" srcset="{{parent_image_mobile}}">{{/if}}
      {{#if parent_image_tablet}}<source media="(max-width: 1199px)" srcset="{{parent_image_tablet}}">{{/if}}
      {{#if parent_image_desktop}}<source media="(min-width: 1200px)" srcset="{{parent_image_desktop}}">{{/if}}
      <img src="{{parent_image_url}}" alt="{{parent_image_alt}}" class="dd-img">
    </picture>
  </div>
  <style>
    .dd-hero__image {
      background-image: url('{{bg_mobile}}');
    }
    @media only screen and (min-width: 64em) {
      .dd-hero__image {
        background-image: url('{{bg_desktop}}');
      }
    }
  </style>{{/if}}
  <div class="dd-hero__content dd-g" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-hero__copy dd-u-1-1 dd-u-lg-12-24">
      <div class="dd-hero__title"><h1>{{parent_title}}</h1></div>
      {{#if parent_subtitle}}<div class="dd-hero__subtitle"><strong>{{parent_subtitle}}</strong></div>{{/if}}
      {{#if has_body}}<div class="dd-hero__body">
        {{#if parent_copy_html}}{{{parent_copy_html}}}{{/if}}
        {{#if has_links}}<div class="dd-hero__links dd-g">
          {{#if has_link_1}}<div class="dd-hero__link">
            <a href="{{link_1_url}}" target="{{link_1_target}}" class="dd-button -primary">{{link_1_label}}</a>
          </div>{{/if}}
          {{#if has_link_2}}<div class="dd-hero__link">
            <a href="{{link_2_url}}" target="{{link_2_target}}" class="dd-button -ghost">{{link_2_label}}</a>
          </div>{{/if}}
        </div>{{/if}}
      </div>{{/if}}
    </div>
  </div>
</section>"#;
    render_inline(template, hero_to_json(hero))
}

fn render_section(section: &DdSection) -> anyhow::Result<String> {
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
                SectionComponent::Alternating(v) => render_alternating(v)?,
                SectionComponent::Card(v) => render_card(v)?,
                SectionComponent::Cta(v) => render_cta(v)?,
                SectionComponent::Filmstrip(v) => render_filmstrip(v)?,
                SectionComponent::Milestones(v) => render_milestones(v)?,
                SectionComponent::Slider(v) => render_slider(v)?,
                SectionComponent::Modal(v) => render_modal(v)?,
                SectionComponent::Banner(v) => render_banner(v)?,
                SectionComponent::Accordion(v) => render_accordion(v)?,
                SectionComponent::Blockquote(v) => render_blockquote(v)?,
                SectionComponent::Alert(v) => render_alert(v)?,
                SectionComponent::Image(v) => render_image(v)?,
                SectionComponent::RichText(v) => render_rich_text(v)?,
                SectionComponent::Navigation(v) => render_navigation(v)?,
                SectionComponent::HeaderSearch(v) => render_header_search(v)?,
                SectionComponent::HeaderMenu(v) => render_header_menu(v)?,
            };
            inner.push_str(&html);
            inner.push('\n');
        }
        columns_html.push_str(&format!(
            r#"<div class="dd-section__item {} {}">{}</div>"#,
            column.width_class, item_box_class, inner
        ));
        columns_html.push('\n');
    }

    let template = r#"<section class="dd-section {{section_class}}" aria-label="Content section">
  <div class="dd-section__content">
    {{#if section_title}}<div class="dd-section__title l-box">{{section_title}}</div>{{/if}}
    <div class="dd-section__items dd-g">
      {{{content}}}
    </div>
  </div>
</section>"#;

    render_inline(
        template,
        json!({
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

fn render_alternating(alternating: &DdAlternating) -> anyhow::Result<String> {
    let template = r#"<div class="dd-alternating {{parent_type}} {{parent_class}}" role="region">
  <div class="dd-alternating__items dd-g">
    {{#each items}}
    <div class="dd-alternating__item dd-u-1-1">
      <div class="dd-alternating__content dd-g">
        <div class="dd-alternating__image dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="{{../parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <picture>
            <img src="{{child_image_url}}" class="dd-img" alt="{{child_image_alt}}" />
          </picture>
        </div>
        <div class="dd-alternating__copy l-box dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="{{../parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <div class="dd-alternating__title">
            <h2>{{child_title}}</h2>
          </div>
          <div class="dd-alternating__body">
            {{child_copy}}
          </div>
        </div>
      </div>
    </div>
    {{/each}}
  </div>
</div>"#;
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
            "parent_data_aos".to_string(),
            Value::String(
                serde_json::to_value(alternating.parent_data_aos)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade-in".to_string()),
            ),
        );
    }
    render_inline(template, v)
}

fn render_card(card: &DdCard) -> anyhow::Result<String> {
    let template = r#"<div class="dd-card {{parent_type}}">
  <div class="dd-card__items dd-g">
    {{#each items}}
    <div class="dd-card__item l-box {{../parent_width}}" data-aos="{{../parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
      <div class="dd-card__body dd-g">
        <div class="dd-card__image">
          <img src="{{child_image_url}}" alt="{{child_image_alt}}" class="dd-img" loading="lazy">
        </div>
        <div class="dd-card__copy l-box">
          <div class="dd-card__title">
            <h3>{{child_title}}</h3>
          </div>
          <div class="dd-card__subtitle">
            <strong>{{child_subtitle}}</strong>
          </div>
          <p>{{child_copy}}</p>
          {{#if has_link}}
          <div class="dd-card__links dd-g">
            <div class="dd-card__link">
              <a href="{{child_link_url}}" target="{{child_link_target}}" class="dd-button -primary">{{child_link_label}}</a>
            </div>
          </div>
          {{/if}}
        </div>
      </div>
    </div>
    {{/each}}
  </div>
</div>"#;
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
    let data = json!({
        "parent_type": serde_json::to_value(card.parent_type).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "parent_data_aos": serde_json::to_value(card.parent_data_aos).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade-in".to_string()),
        "parent_width": card.parent_width,
        "items": items
    });
    render_inline(template, data)
}

fn render_banner(banner: &DdBanner) -> anyhow::Result<String> {
    let template = r#"<div class="dd-banner {{parent_class}}" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100" style="background-image: url({{parent_image_url}});">
  <div class="dd-banner__image">
    <picture>
      <img src="{{parent_image_url}}" class="dd-img" alt="{{parent_image_alt}}" />
    </picture>
  </div>
</div>"#;
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
            "parent_data_aos".to_string(),
            Value::String(
                serde_json::to_value(banner.parent_data_aos)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade-in".to_string()),
            ),
        );
    }
    render_inline(template, v)
}

fn render_cta(cta: &DdCta) -> anyhow::Result<String> {
    let template = r#"<div class="dd-cta {{parent_class}}">
  <div class="dd-cta__image" style="background-image: url({{parent_image_url}});">
    <picture>
      <img src="{{parent_image_url}}" class="dd-img" alt="{{parent_image_alt}}" />
    </picture>
  </div>
  <div class="dd-cta__content dd-g" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-center" data-aos-delay="100">
    <div class="dd-cta__copy dd-u-1-1 dd-u-md-12-24">
      <div class="dd-cta__title">
        <h2>{{parent_title}}</h2>
      </div>
      <div class="dd-cta__subtitle">
        <strong>{{parent_subtitle}}</strong>
      </div>
      <p>{{parent_copy}}</p>
      {{#if has_link}}
      <div class="dd-cta__links dd-g -x-center">
        <div class="dd-cta__link">
          <a href="{{parent_link_url}}" class="dd-button -primary" target="{{parent_link_target}}">{{parent_link_label}}</a>
        </div>
      </div>
      {{/if}}
    </div>
  </div>
</div>"#;

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
        "parent_data_aos": serde_json::to_value(cta.parent_data_aos).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade-in".to_string()),
        "parent_title": cta.parent_title,
        "parent_subtitle": cta.parent_subtitle,
        "parent_copy": cta.parent_copy,
        "parent_link_url": link_url.unwrap_or_default(),
        "parent_link_target": link_target,
        "parent_link_label": link_label.unwrap_or_default(),
        "has_link": has_link
    });
    render_inline(template, data)
}

fn render_filmstrip(filmstrip: &DdFilmstrip) -> anyhow::Result<String> {
    let template = r#"<div class="dd-filmstrip {{parent_type}}" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-center" data-aos-delay="100">
  <ul class="dd-filmstrip__content">
    {{#each items}}
    <li>
      <img src="{{child_image_url}}" alt="{{child_image_alt}}" class="dd-img" loading="lazy">
      <figure class="dd-filmstrip__title">{{child_title}}</figure>
    </li>
    {{/each}}
  </ul>

  <ul aria-hidden="true" class="dd-filmstrip__content">
    {{#each items}}
    <li role="presentation">
      <img src="{{child_image_url}}" alt="{{child_image_alt}}" class="dd-img" loading="lazy">
      <figure class="dd-filmstrip__title">{{child_title}}</figure>
    </li>
    {{/each}}
  </ul>
</div>"#;

    let data = json!({
        "parent_type": serde_json::to_value(filmstrip.parent_type).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "parent_data_aos": serde_json::to_value(filmstrip.parent_data_aos).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade-in".to_string()),
        "items": filmstrip.items
    });
    render_inline(template, data)
}

fn render_milestones(milestones: &DdMilestones) -> anyhow::Result<String> {
    let template = r#"<div class="dd-milestones">
  <div class="dd-milestones__content">
    <div class="dd-milestones__items dd-g">
      {{#each items}}
      <div class="dd-milestones__item l-box {{../parent_width}}" data-aos="{{../parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-center" data-aos-delay="100">
        <div class="dd-milestones__body l-box">
          <div class="dd-milestones__percentage" data-number="{{child_percentage}}"><span class="number">{{child_percentage}}</span>%</div>
          <div>
            <div class="dd-milestones__title"><h2>{{child_title}}</h2></div>
            <div class="dd-milestones__subtitle"><strong>{{child_subtitle}}</strong></div>
            <div class="dd-milestones__copy">{{child_copy}}</div>
            {{#if has_link}}
            <div class="dd-milestones__links">
              <div class="dd-milestones__link">
                <a href="{{child_link_url}}" target="{{child_link_target}}" class="dd-button -primary">{{child_link_label}}</a>
              </div>
            </div>
            {{/if}}
          </div>
        </div>
      </div>
      {{/each}}
    </div>
  </div>
</div>"#;
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
    let data = json!({
        "parent_data_aos": serde_json::to_value(milestones.parent_data_aos).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade-in".to_string()),
        "parent_width": milestones.parent_width,
        "items": items
    });
    render_inline(template, data)
}

fn render_modal(modal: &DdModal) -> anyhow::Result<String> {
    let template = r#"<button class="dd-modal__button-open" data-modal-open data-id="{{parent_modal_id}}">{{parent_title}}</button>
<dialog data-modal id="{{parent_modal_id}}" class="dd-modal">
  <button class="dd-modal__button-close" data-modal-close data-id="{{parent_modal_id}}" aria-label="close modal window">X</button>
  <div class="dd-modal__copy">
    <p>{{parent_copy}}</p>
  </div>
</dialog>"#;
    let data = json!({
        "parent_title": modal.parent_title,
        "parent_copy": modal.parent_copy,
        "parent_modal_id": html_id_safe_from_title(&modal.parent_title, "modal")
    });
    render_inline(template, data)
}

fn render_slider(slider: &DdSlider) -> anyhow::Result<String> {
    let template = r#"<div class="dd-slider">
  {{#if has_parent_title}}
  <div class="dd-slider__title">
    <h2>{{parent_title}}</h2>
  </div>
  {{/if}}
  <ul class="dd-slider__items -nostyle">
    {{#each items}}
    <li class="dd-slider__item" data-id="{{../parent_uid}}">
      <div class="dd-slider__content">
        <div class="dd-g">
          <div class="dd-slider__body dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24 l-box">
            <div class="dd-slider__title">
              {{child_title}}
            </div>
            <div class="dd-slider__copy">
              {{child_copy}}
              {{#if has_link}}
              <div class="dd-slider__links">
                <div class="dd-slider__link">
                  <a href="{{child_link_url}}" target="{{child_link_target}}" class="dd-button -primary">{{child_link_label}}</a>
                </div>
              </div>
              {{/if}}
            </div>
          </div>
          <div class="dd-slider__image dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24">
            <img src="{{child_image_url}}" alt="{{child_image_alt}}" />
          </div>
        </div>
      </div>
    </li>
    {{/each}}
  </ul>
  <div class="dd-slider__navigation">
    <button id="dd-slider__previous"><span class="-scrn-reader-only">Previous slide</span> &lt; </button>
    <ul class="dd-slider__tabs -nostyle"></ul>
    <button id="dd-slider__next"><span class="-scrn-reader-only">Next slide</span> &gt; </button>
  </div>
</div>"#;

    let fallback_uid = random_uid_fallback();
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
    render_inline(template, data)
}

fn render_accordion(accordion: &DdAccordion) -> anyhow::Result<String> {
    let template = r#"<div class="dd-accordion {{parent_type}} {{parent_class}}" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <div class="dd-accordion__items">
    {{#each items}}<details name="{{../parent_group_name}}" class="dd-accordion__item">
      <summary class="dd-accordion__header dd-g -y-center">
        <div class="dd-accordion__title dd-u-1-1">{{child_title}}</div>
      </summary>
      <div class="dd-accordion__copy"><p>{{child_copy}}</p></div>
    </details>
    {{/each}}
  </div>
</div>{{#if has_faq_schema}}
<script type="application/ld+json">{{{faq_schema_json}}}</script>{{/if}}"#;
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
            "parent_data_aos".to_string(),
            Value::String(
                serde_json::to_value(accordion.parent_data_aos)
                    .map(|v| stringify_json(&v))
                    .unwrap_or_else(|_| "fade-in".to_string()),
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
    render_inline(template, v)
}

fn render_blockquote(blockquote: &DdBlockquote) -> anyhow::Result<String> {
    let template = r#"<blockquote class="dd-blockquote">
  <div class="dd-blockquote__content dd-g" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-blockquote__icon"><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-quote-icon lucide-quote"><path d="M16 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/><path d="M5 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/></svg></div>
    <div class="dd-blockquote__person dd-g l-box">
      <div class="dd-blockquote__image">
        <picture>
          <img src="{{parent_image_url}}" class="dd-img" alt="{{parent_image_alt}}" />
        </picture>
      </div>
      <div class="dd-blockquote__name-title">
        <span class="dd-blockquote__name">{{parent_name}}</span>
        <span class="dd-blockquote__title">, {{parent_role}}</span>
      </div>
      <div class="dd-blockquote__comment">
        {{parent_copy}}
      </div>
    </div>
  </div>
</blockquote>
<script type="application/ld+json">{{{blockquote_schema_json}}}</script>"#;
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
            "parent_data_aos".to_string(),
            Value::String(
                serde_json::to_value(blockquote.parent_data_aos)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade-in".to_string()),
            ),
        );
        obj.insert(
            "blockquote_schema_json".to_string(),
            Value::String(blockquote_schema_json),
        );
    }
    render_inline(template, v)
}

fn render_alert(alert: &DdAlert) -> anyhow::Result<String> {
    let template = r#"<div class="dd-alert {{parent_type}} {{parent_class}}" role="alert" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <div class="dd-alert__content dd-g">
    <div class="dd-u-1-1">
      <div class="l-box">
        {{#if has_title}}<div class="dd-alert__title">{{parent_title}}</div>{{/if}}
        <div class="dd-alert__copy">
          <p>{{parent_copy}}</p>
        </div>
      </div>
    </div>
  </div>
</div>"#;
    let data = json!({
        "parent_type": serde_json::to_value(alert.parent_type).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "parent_class": serde_json::to_value(alert.parent_class).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "parent_data_aos": serde_json::to_value(alert.parent_data_aos).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade-in".to_string()),
        "parent_title": alert.parent_title.as_deref().unwrap_or(""),
        "has_title": alert.parent_title.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false),
        "parent_copy": alert.parent_copy
    });
    render_inline(template, data)
}

fn render_image(image: &crate::model::DdImage) -> anyhow::Result<String> {
    let data_aos = aos_token(image.parent_data_aos);
    let has_link = image
        .parent_link_url
        .as_deref()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let template = if has_link {
        r#"<div class="dd-image" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <a href="{{parent_link_url}}" target="{{parent_link_target}}" title="{{parent_image_alt}}">
    <img src="{{parent_image_url}}" alt="{{parent_image_alt}}" class="dd-img" loading="lazy" />
  </a>
</div>"#
    } else {
        r#"<div class="dd-image" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <img src="{{parent_image_url}}" alt="{{parent_image_alt}}" class="dd-img" loading="lazy" />
</div>"#
    };
    let link_target = image
        .parent_link_target
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "_self".to_string());
    let data = json!({
        "parent_data_aos": data_aos,
        "parent_image_url": image.parent_image_url,
        "parent_image_alt": image.parent_image_alt,
        "parent_link_url": image.parent_link_url.clone().unwrap_or_default(),
        "parent_link_target": link_target,
    });
    render_inline(template, data)
}

fn render_rich_text(rt: &crate::model::DdRichText) -> anyhow::Result<String> {
    let template = r#"<div class="dd-rich_text{{#if parent_class}} {{parent_class}}{{/if}}" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <div class="dd-rich_text__copy">{{{parent_copy_html}}}</div>
</div>"#;
    let parent_class = rt
        .parent_class
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let parent_copy_html = markdown_to_html(&rt.parent_copy);
    let data = json!({
        "parent_class": parent_class,
        "parent_data_aos": aos_token(rt.parent_data_aos),
        "parent_copy_html": parent_copy_html,
    });
    render_inline(template, data)
}

fn render_navigation(nav: &crate::model::DdNavigation) -> anyhow::Result<String> {
    let parent_class = navigation_class_token(nav.parent_class);
    let aria_label = match nav.parent_type {
        crate::model::NavigationType::HeaderNav => "header navigation",
        crate::model::NavigationType::FooterNav => "footer navigation",
    };
    let items_html = render_nav_items(&nav.items);
    Ok(format!(
        r#"<div class="dd-navigation {parent_class} -y-center" data-aos="{aos}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <nav itemscope itemtype="https://schema.org/SiteNavigationElement" aria-label="{aria_label}">
    <button class="dd-menu__close fa-regular fa-times" type="button"><span class="visually-hidden">Menu</span></button>
    <ul class="menu">
{items_html}
    </ul>
  </nav>
</div>"#,
        parent_class = parent_class,
        aos = aos_token(nav.parent_data_aos),
        aria_label = aria_label,
        items_html = items_html,
    ))
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

fn render_header_search(search: &crate::model::DdHeaderSearch) -> anyhow::Result<String> {
    let template = r#"<div class="dd-header__search-icon {{parent_width}} -y-center -x-center" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <button class="dd-search__toggle fa-regular fa-magnifying-glass" type="button">
    <span class="visually-hidden">Search</span>
  </button>
</div>"#;
    let data = json!({
        "parent_width": search.parent_width,
        "parent_data_aos": aos_token(search.parent_data_aos),
    });
    render_inline(template, data)
}

fn render_header_menu(menu: &crate::model::DdHeaderMenu) -> anyhow::Result<String> {
    let template = r#"<div class="dd-header__menu-icon {{parent_width}} -y-center -x-center" data-aos="{{parent_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <button class="dd-menu__toggle fa-regular fa-bars" type="button">
    <span class="visually-hidden">Menu</span>
  </button>
</div>"#;
    let data = json!({
        "parent_width": menu.parent_width,
        "parent_data_aos": aos_token(menu.parent_data_aos),
    });
    render_inline(template, data)
}

fn aos_token(aos: crate::model::HeroAos) -> String {
    serde_json::to_value(aos)
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|_| "fade-in".to_string())
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

fn render_inline(template: &str, data: Value) -> anyhow::Result<String> {
    let mut hbs = Handlebars::new();
    hbs.register_template_string("inline", template)
        .context("failed to register inline template")?;
    hbs.render("inline", &data)
        .context("failed to render inline template")
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
    let parent_data_aos = hero
        .parent_data_aos
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "fade-in".to_string());
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
        "parent_image_url": hero.parent_image_url,
        "parent_class": parent_class,
        "parent_data_aos": parent_data_aos,
        "parent_custom_css": parent_custom_css,
        "parent_title": hero.parent_title,
        "parent_subtitle": if subtitle.is_empty() { None } else { Some(hero.parent_subtitle.clone()) },
        "parent_copy_html": parent_copy_html,
        "link_1_label": hero.link_1_label,
        "link_1_url": hero.link_1_url,
        "link_1_target": link_1_target,
        "link_2_label": hero.link_2_label,
        "link_2_url": hero.link_2_url,
        "link_2_target": link_2_target,
        "parent_image_alt": hero.parent_image_alt.clone().unwrap_or_default(),
        "parent_image_mobile": hero.parent_image_mobile,
        "parent_image_tablet": hero.parent_image_tablet,
        "parent_image_desktop": hero.parent_image_desktop,
        "parent_image_class": parent_image_class,
        "has_image": has_image,
        "has_body": has_body,
        "has_links": has_link_1 || has_link_2,
        "has_link_1": has_link_1,
        "has_link_2": has_link_2,
        "bg_mobile": bg_mobile,
        "bg_desktop": bg_desktop
    })
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

fn random_uid_fallback() -> String {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let n = seed % 1_000_000;
    format!("uid-{n:06}")
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
