use std::fs;
use std::path::Path;

use anyhow::Context;
use handlebars::Handlebars;
use serde_json::{Value, json};

use crate::model::{
    DdAccordion, DdAlternating, DdBanner, DdBlockquote, DdCard, DdHero, DdSection, Page, PageNode,
    SectionColumn, SectionComponent, Site,
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
    let template = r#"<section class="dd-hero{{#if hero_class}} {{hero_class}}{{/if}}{{#if custom_css}} {{custom_css}}{{/if}}" aria-label="Introduction">
  {{#if has_image}}<div class="dd-hero__image {{image_class}}">
    <picture>
      {{#if image_mobile}}<source media="(max-width: 767px)" srcset="{{image_mobile}}">{{/if}}
      {{#if image_tablet}}<source media="(max-width: 1199px)" srcset="{{image_tablet}}">{{/if}}
      {{#if image_desktop}}<source media="(min-width: 1200px)" srcset="{{image_desktop}}">{{/if}}
      <img src="{{image}}" alt="{{image_alt}}" class="dd-img">
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
  <div class="dd-hero__content dd-g" data-aos="{{hero_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-hero__copy dd-u-1-1 dd-u-lg-12-24">
      <div class="dd-hero__title"><h1>{{title}}</h1></div>
      {{#if subtitle}}<div class="dd-hero__subtitle"><strong>{{subtitle}}</strong></div>{{/if}}
      {{#if has_body}}<div class="dd-hero__body">
        {{#if copy_html}}{{{copy_html}}}{{/if}}
        {{#if has_links}}<div class="dd-hero__links dd-g">
          {{#if has_primary_cta}}<div class="dd-hero__link">
            <a href="{{cta_link}}" target="{{cta_target}}" class="dd-button -primary">{{cta_text}}</a>
          </div>{{/if}}
          {{#if has_secondary_cta}}<div class="dd-hero__link">
            <a href="{{cta_link_2}}" target="{{cta_target_2}}" class="dd-button -ghost">{{cta_text_2}}</a>
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
    for column in section_columns(section) {
        let mut inner = String::new();
        for component in &column.components {
            let html = match component {
                SectionComponent::Alternating(v) => render_alternating(v)?,
                SectionComponent::Card(v) => render_card(v)?,
                SectionComponent::Banner(v) => render_banner(v)?,
                SectionComponent::Accordion(v) => render_accordion(v)?,
                SectionComponent::Blockquote(v) => render_blockquote(v)?,
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

fn render_alternating(alternating: &DdAlternating) -> anyhow::Result<String> {
    let template = r#"<div class="dd-alternating {{alternating_type}} {{alternating_class}}" role="region">
  <div class="dd-alternating__items dd-g">
    {{#each items}}
    <div class="dd-alternating__item dd-u-1-1">
      <div class="dd-alternating__content dd-g">
        <div class="dd-alternating__image dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="{{../alternating_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <picture>
            <img src="{{image}}" class="dd-img" alt="{{image_alt}}" />
          </picture>
        </div>
        <div class="dd-alternating__copy l-box dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-12-24" data-aos="{{../alternating_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
          <div class="dd-alternating__title">
            <h2>{{title}}</h2>
          </div>
          <div class="dd-alternating__body">
            {{copy}}
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
            "alternating_type".to_string(),
            Value::String(
                serde_json::to_value(alternating.alternating_type)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "-default".to_string()),
            ),
        );
        obj.insert(
            "alternating_data_aos".to_string(),
            Value::String(
                serde_json::to_value(alternating.alternating_data_aos)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade-in".to_string()),
            ),
        );
    }
    render_inline(template, v)
}

fn render_card(card: &DdCard) -> anyhow::Result<String> {
    let template = r#"<div class="dd-card {{card_type}}">
  <div class="dd-card__items dd-g">
    {{#each items}}
    <div class="dd-card__item l-box {{../card_width}}" data-aos="{{../card_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
      <div class="dd-card__body dd-g">
        <div class="dd-card__image">
          <img src="{{card_image_url}}" alt="{{card_image_alt}}" class="dd-img" loading="lazy">
        </div>
        <div class="dd-card__copy l-box">
          <div class="dd-card__title">
            <h3>{{card_title}}</h3>
          </div>
          <div class="dd-card__subtitle">
            <strong>{{card_subtitle}}</strong>
          </div>
          <p>{{card_copy}}</p>
          {{#if has_link}}
          <div class="dd-card__links dd-g">
            <div class="dd-card__link">
              <a href="{{card_link_url}}" target="{{card_link_target}}" class="dd-button -primary">{{card_link_label}}</a>
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
            .card_link_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let link_label = item
            .card_link_label
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);
        let has_link = link_url.is_some() && link_label.is_some();
        let link_target = item
            .card_link_target
            .as_ref()
            .and_then(|v| serde_json::to_value(v).ok())
            .map(|v| stringify_json(&v))
            .unwrap_or_else(|| "_self".to_string());
        items.push(json!({
            "card_image_url": item.card_image_url,
            "card_image_alt": item.card_image_alt,
            "card_title": item.card_title,
            "card_subtitle": item.card_subtitle,
            "card_copy": item.card_copy,
            "card_link_url": link_url.unwrap_or_default(),
            "card_link_target": link_target,
            "card_link_label": link_label.unwrap_or_default(),
            "has_link": has_link
        }));
    }
    let data = json!({
        "card_type": serde_json::to_value(card.card_type).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "-default".to_string()),
        "card_data_aos": serde_json::to_value(card.card_data_aos).map(|raw| stringify_json(&raw)).unwrap_or_else(|_| "fade-in".to_string()),
        "card_width": card.card_width,
        "items": items
    });
    render_inline(template, data)
}

fn render_banner(banner: &DdBanner) -> anyhow::Result<String> {
    let template = r#"<div class="dd-banner {{banner_class}}" data-aos="{{banner_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100" style="background-image: url({{banner_image_url}});">
  <div class="dd-banner__image">
    <picture>
      <img src="{{banner_image_url}}" class="dd-img" alt="{{banner_image_alt}}" />
    </picture>
  </div>
</div>"#;
    let mut v = serde_json::to_value(banner)?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "banner_class".to_string(),
            Value::String(
                serde_json::to_value(banner.banner_class)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "-bg-center-center".to_string()),
            ),
        );
        obj.insert(
            "banner_data_aos".to_string(),
            Value::String(
                serde_json::to_value(banner.banner_data_aos)
                    .map(|raw| stringify_json(&raw))
                    .unwrap_or_else(|_| "fade-in".to_string()),
            ),
        );
    }
    render_inline(template, v)
}

fn render_accordion(accordion: &DdAccordion) -> anyhow::Result<String> {
    let template = r#"<div class="dd-accordion {{accordion_type}} {{accordion_class}}" data-aos="{{accordion_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
  <div class="dd-accordion__items">
    {{#each items}}<details name="{{../group_name}}" class="dd-accordion__item">
      <summary class="dd-accordion__header dd-g -y-center">
        <div class="dd-accordion__title dd-u-1-1">{{title}}</div>
      </summary>
      <div class="dd-accordion__copy"><p>{{content}}</p></div>
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
                "name": item.title,
                "acceptedAnswer": {
                    "@type": "Answer",
                    "text": item.content
                }
            })
        }).collect::<Vec<_>>()
    }))?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "accordion_type".to_string(),
            Value::String(
                serde_json::to_value(accordion.accordion_type)
                    .map(|v| stringify_json(&v))
                    .unwrap_or_else(|_| "-default".to_string()),
            ),
        );
        obj.insert(
            "accordion_class".to_string(),
            Value::String(
                serde_json::to_value(accordion.accordion_class)
                    .map(|v| stringify_json(&v))
                    .unwrap_or_else(|_| "-primary".to_string()),
            ),
        );
        obj.insert(
            "accordion_aos".to_string(),
            Value::String(
                serde_json::to_value(accordion.accordion_aos)
                    .map(|v| stringify_json(&v))
                    .unwrap_or_else(|_| "fade-in".to_string()),
            ),
        );
        obj.insert(
            "has_faq_schema".to_string(),
            Value::Bool(matches!(
                accordion.accordion_type,
                crate::model::AccordionType::Faq
            )),
        );
        obj.insert("faq_schema_json".to_string(), Value::String(faq_schema));
    }
    render_inline(template, v)
}

fn render_blockquote(blockquote: &DdBlockquote) -> anyhow::Result<String> {
    let template = r#"<blockquote class="dd-blockquote">
  <div class="dd-blockquote__content dd-g" data-aos="{{blockquote_data_aos}}" data-aos-duration="1000" data-aos-easing="linear" data-aos-anchor-placement="center-bottom" data-aos-delay="100">
    <div class="dd-blockquote__icon"><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-quote-icon lucide-quote"><path d="M16 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/><path d="M5 3a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2 1 1 0 0 1 1 1v1a2 2 0 0 1-2 2 1 1 0 0 0-1 1v2a1 1 0 0 0 1 1 6 6 0 0 0 6-6V5a2 2 0 0 0-2-2z"/></svg></div>
    <div class="dd-blockquote__person dd-g l-box">
      <div class="dd-blockquote__image">
        <picture>
          <img src="{{blockquote_image_url}}" class="dd-img" alt="{{blockquote_image_alt}}" />
        </picture>
      </div>
      <div class="dd-blockquote__name-title">
        <span class="dd-blockquote__name">{{blockquote_persons_name}}</span>
        <span class="dd-blockquote__title">, {{blockquote_persons_title}}</span>
      </div>
      <div class="dd-blockquote__comment">
        {{blockquote_copy}}
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
            blockquote.blockquote_persons_name, blockquote.blockquote_persons_title
        )
      },
      "text": blockquote.blockquote_copy
    }))?;
    let mut v = serde_json::to_value(blockquote)?;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "blockquote_data_aos".to_string(),
            Value::String(
                serde_json::to_value(blockquote.blockquote_data_aos)
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
    let cta_target_2 = hero
        .cta_target_2
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "_self".to_string());
    let image = hero.image.trim();
    let subtitle = hero.subtitle.trim();
    let hero_class = hero
        .hero_class
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v));
    let hero_aos = hero
        .hero_aos
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "fade-in".to_string());
    let custom_css = hero
        .custom_css
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let copy_html = hero
        .copy
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .map(markdown_to_html);
    let has_primary_cta = hero
        .cta_text
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
        && hero
            .cta_link
            .as_deref()
            .is_some_and(|v| !v.trim().is_empty());
    let has_secondary_cta = hero
        .cta_text_2
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
        && hero
            .cta_link_2
            .as_deref()
            .is_some_and(|v| !v.trim().is_empty());
    let bg_mobile = hero
        .image_mobile
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(image);
    let bg_desktop = hero
        .image_desktop
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(image);
    let image_class = hero
        .image_class
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .map(|v| stringify_json(&v))
        .unwrap_or_else(|| "-full-full".to_string());
    let has_image = !image.is_empty();
    let has_body = hero.copy.as_deref().is_some_and(|v| !v.trim().is_empty())
        || has_primary_cta
        || has_secondary_cta;

    json!({
        "image": hero.image,
        "hero_class": hero_class,
        "hero_aos": hero_aos,
        "custom_css": custom_css,
        "title": hero.title,
        "subtitle": if subtitle.is_empty() { None } else { Some(hero.subtitle.clone()) },
        "copy_html": copy_html,
        "cta_text": hero.cta_text,
        "cta_link": hero.cta_link,
        "cta_target": cta_target,
        "cta_text_2": hero.cta_text_2,
        "cta_link_2": hero.cta_link_2,
        "cta_target_2": cta_target_2,
        "image_alt": hero.image_alt.clone().unwrap_or_default(),
        "image_mobile": hero.image_mobile,
        "image_tablet": hero.image_tablet,
        "image_desktop": hero.image_desktop,
        "image_class": image_class,
        "has_image": has_image,
        "has_body": has_body,
        "has_links": has_primary_cta || has_secondary_cta,
        "has_primary_cta": has_primary_cta,
        "has_secondary_cta": has_secondary_cta,
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
