use crate::model::{DdSection, PageNode, SectionColumn, SectionComponent, Site};

pub fn validate_site(site: &Site) -> Vec<String> {
    let mut errors = Vec::new();
    let mut slugs = std::collections::HashSet::new();

    if site.pages.is_empty() {
        errors.push("Site must include at least one page.".to_string());
    }

    for page in &site.pages {
        if page.slug.trim().is_empty() {
            errors.push(format!("Page '{}' has an empty slug.", page.id));
        }
        if !page.slug.trim().is_empty() && !slugs.insert(page.slug.clone()) {
            errors.push(format!("Duplicate page slug '{}'.", page.slug));
        }
        if page.nodes.is_empty() {
            errors.push(format!("Page '{}' has no page nodes.", page.id));
        }
        let mut section_ids = std::collections::HashSet::new();

        for node in &page.nodes {
            match node {
                PageNode::Hero(hero) => {
                    if hero.title.trim().is_empty() {
                        errors.push(format!("Page '{}' hero is missing title.", page.id));
                    }
                    if !hero.image.trim().is_empty()
                        && hero.image_alt.as_deref().unwrap_or("").trim().is_empty()
                    {
                        errors.push(format!(
                            "Page '{}' hero image is missing image_alt text.",
                            page.id
                        ));
                    }
                    if hero.cta_text.is_some() ^ hero.cta_link.is_some() {
                        errors.push(format!(
                            "Page '{}' hero must provide both cta_text and cta_link together.",
                            page.id
                        ));
                    }
                    if let Some(link) = &hero.cta_link {
                        if !is_valid_url(link) {
                            errors.push(format!("Page '{}' hero CTA link is invalid.", page.id));
                        }
                    }
                    if hero.cta_text_2.is_some() ^ hero.cta_link_2.is_some() {
                        errors.push(format!(
                            "Page '{}' hero must provide both cta_text_2 and cta_link_2 together.",
                            page.id
                        ));
                    }
                    if let Some(link) = &hero.cta_link_2 {
                        if !is_valid_url(link) {
                            errors.push(format!(
                                "Page '{}' hero secondary CTA link is invalid.",
                                page.id
                            ));
                        }
                    }
                }
                PageNode::Section(section) => {
                    if section.id.trim().is_empty() {
                        errors.push(format!("Page '{}' has section with empty id.", page.id));
                    } else if !section_ids.insert(section.id.clone()) {
                        errors.push(format!(
                            "Page '{}' has duplicate section id '{}'.",
                            page.id, section.id
                        ));
                    }
                    let columns = section_columns(section);
                    if columns.is_empty() {
                        errors.push(format!("Section '{}' has no columns.", section.id));
                    }
                    let mut column_ids = std::collections::HashSet::new();
                    for column in &columns {
                        if column.id.trim().is_empty() {
                            errors.push(format!(
                                "Page '{}' section '{}' has a column with empty id.",
                                page.id, section.id
                            ));
                        } else if !column_ids.insert(column.id.clone()) {
                            errors.push(format!(
                                "Page '{}' section '{}' has duplicate column id '{}'.",
                                page.id, section.id, column.id
                            ));
                        }
                        if column.width_class.trim().is_empty() {
                            errors.push(format!(
                                "Page '{}' section '{}' column '{}' missing width_class.",
                                page.id, section.id, column.id
                            ));
                        }
                        for component in &column.components {
                            validate_section_component(
                                component,
                                page.id.as_str(),
                                section.id.as_str(),
                                &mut errors,
                            );
                        }
                    }
                }
            }
        }
    }

    errors
}

fn validate_section_component(
    component: &SectionComponent,
    page_id: &str,
    section_id: &str,
    errors: &mut Vec<String>,
) {
    match component {
        SectionComponent::Alternating(alternating) => {
            if alternating.items.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-alternating with no items.",
                    page_id, section_id
                ));
            }
            for (idx, item) in alternating.items.iter().enumerate() {
                if item.image.trim().is_empty()
                    || item.image_alt.trim().is_empty()
                    || item.title.trim().is_empty()
                    || item.copy.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-alternating item {} has missing required fields.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Card(card) => {
            if card.card_width.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-card with empty card_width.",
                    page_id, section_id
                ));
            }
            if card.items.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-card with no items.",
                    page_id, section_id
                ));
            }
            for (idx, item) in card.items.iter().enumerate() {
                if item.card_image_url.trim().is_empty()
                    || item.card_image_alt.trim().is_empty()
                    || item.card_title.trim().is_empty()
                    || item.card_subtitle.trim().is_empty()
                    || item.card_copy.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} has missing required fields.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if !is_valid_url(&item.card_image_url) {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} card_image_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                let has_link_url = item
                    .card_link_url
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                let has_link_label = item
                    .card_link_label
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                if has_link_url ^ has_link_label {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} must provide both card_link_url and card_link_label together.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if let Some(url) = item.card_link_url.as_deref()
                    && !url.trim().is_empty()
                    && !is_valid_url(url)
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} card_link_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Banner(banner) => {
            if banner.banner_image_url.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-banner with empty banner_image_url.",
                    page_id, section_id
                ));
            }
            if banner.banner_image_alt.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-banner with empty banner_image_alt.",
                    page_id, section_id
                ));
            }
            if !is_valid_url(&banner.banner_image_url) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-banner banner_image_url is invalid.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Accordion(accordion) => {
            if accordion.group_name.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-accordion missing group_name.",
                    page_id, section_id
                ));
            }
            if accordion.items.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-accordion with no items.",
                    page_id, section_id
                ));
            }
            for (idx, item) in accordion.items.iter().enumerate() {
                if item.title.trim().is_empty() || item.content.trim().is_empty() {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-accordion item {} has missing title/content.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Blockquote(blockquote) => {
            if blockquote.blockquote_image_url.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-blockquote with empty blockquote_image_url.",
                    page_id, section_id
                ));
            }
            if blockquote.blockquote_image_alt.trim().is_empty()
                || blockquote.blockquote_persons_name.trim().is_empty()
                || blockquote.blockquote_persons_title.trim().is_empty()
                || blockquote.blockquote_copy.trim().is_empty()
            {
                errors.push(format!(
                    "Page '{}' section '{}' dd-blockquote has missing required fields.",
                    page_id, section_id
                ));
            }
            if !is_valid_url(&blockquote.blockquote_image_url) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-blockquote blockquote_image_url is invalid.",
                    page_id, section_id
                ));
            }
        }
    }
}

fn is_valid_url(url: &str) -> bool {
    let v = url.trim();
    !v.is_empty()
        && (v.starts_with('/')
            || v.starts_with('#')
            || v.starts_with("http://")
            || v.starts_with("https://"))
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

#[cfg(test)]
mod tests {
    use super::validate_site;
    use crate::model::{PageNode, Site};

    #[test]
    fn starter_site_is_valid() {
        let site = Site::starter();
        let errors = validate_site(&site);
        assert!(
            errors.is_empty(),
            "expected no validation errors, got {errors:?}"
        );
    }

    #[test]
    fn detects_missing_hero_required_fields() {
        let mut site = Site::starter();
        let page = &mut site.pages[0];
        if let PageNode::Hero(hero) = &mut page.nodes[0] {
            hero.title.clear();
            hero.subtitle.clear();
            hero.image.clear();
        }
        let errors = validate_site(&site);
        assert!(errors.iter().any(|e| e.contains("missing title")));
        assert!(!errors.iter().any(|e| e.contains("missing subtitle")));
        assert!(!errors.iter().any(|e| e.contains("missing image")));
    }

    #[test]
    fn detects_duplicate_page_slug() {
        let mut site = Site::starter();
        site.pages.push(site.pages[0].clone());
        let errors = validate_site(&site);
        assert!(errors.iter().any(|e| e.contains("Duplicate page slug")));
    }
}
