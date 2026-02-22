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
        SectionComponent::Card(card) => {
            if card.title.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' has a dd-card with empty title.",
                    page_id
                ));
            }
            if card.image.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-card with empty image.",
                    page_id, section_id
                ));
            }
            if !card.image.trim().is_empty()
                && card.image_alt.as_deref().unwrap_or("").trim().is_empty()
            {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-card missing image_alt text.",
                    page_id, section_id
                ));
            }
            if card.cta_text.is_some() ^ card.cta_link.is_some() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-card must provide both cta_text and cta_link together.",
                    page_id, section_id
                ));
            }
            if let Some(link) = &card.cta_link {
                if !is_valid_url(link) {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card CTA link is invalid.",
                        page_id, section_id
                    ));
                }
            }
        }
        SectionComponent::Alert(alert) => {
            if alert.message.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-alert with empty message.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Banner(banner) => {
            if banner.message.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-banner with empty message.",
                    page_id, section_id
                ));
            }
            if banner.background.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-banner with empty background.",
                    page_id, section_id
                ));
            }
            if banner.link_text.is_some() ^ banner.link_url.is_some() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-banner must provide both link_text and link_url together.",
                    page_id, section_id
                ));
            }
            if let Some(link) = &banner.link_url {
                if !is_valid_url(link) {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-banner link_url is invalid.",
                        page_id, section_id
                    ));
                }
            }
        }
        SectionComponent::Tabs(tabs) => {
            if tabs.tabs.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-tabs with no tab items.",
                    page_id, section_id
                ));
            }
            for (idx, tab) in tabs.tabs.iter().enumerate() {
                if tab.title.trim().is_empty() || tab.content.trim().is_empty() {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-tabs item {} has missing title/content.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
            if let Some(default_idx) = tabs.default_tab {
                if default_idx >= tabs.tabs.len() {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-tabs default_tab is out of range.",
                        page_id, section_id
                    ));
                }
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
        SectionComponent::Cta(cta) => {
            if cta.title.trim().is_empty()
                || cta.copy.trim().is_empty()
                || cta.cta_text.trim().is_empty()
                || cta.cta_link.trim().is_empty()
            {
                errors.push(format!(
                    "Page '{}' section '{}' has invalid dd-cta required fields.",
                    page_id, section_id
                ));
            } else if !is_valid_url(&cta.cta_link) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-cta cta_link is invalid.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Modal(modal) => {
            if modal.trigger_text.trim().is_empty()
                || modal.title.trim().is_empty()
                || modal.content.trim().is_empty()
            {
                errors.push(format!(
                    "Page '{}' section '{}' has invalid dd-modal required fields.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Slider(slider) => {
            if slider.slides.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-slider with no slides.",
                    page_id, section_id
                ));
            }
            for (idx, slide) in slider.slides.iter().enumerate() {
                if slide.image.trim().is_empty()
                    || slide.title.trim().is_empty()
                    || slide.copy.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-slider slide {} has missing image/title/copy.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
            if slider.speed == Some(0) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-slider speed must be greater than zero.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Spacer(_) => {}
        SectionComponent::Timeline(timeline) => {
            if timeline.events.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-timeline with no events.",
                    page_id, section_id
                ));
            }
            for (idx, event) in timeline.events.iter().enumerate() {
                if event.date.trim().is_empty()
                    || event.title.trim().is_empty()
                    || event.description.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-timeline event {} has missing date/title/description.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
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

    #[test]
    fn detects_invalid_cta_url() {
        let mut site = Site::starter();
        let page = &mut site.pages[0];
        if let PageNode::Section(section) = &mut page.nodes[1] {
            section.columns[0]
                .components
                .push(crate::model::SectionComponent::Cta(crate::model::DdCta {
                    title: "Test CTA".to_string(),
                    copy: "Test copy".to_string(),
                    cta_text: "Go".to_string(),
                    cta_link: "bad-link".to_string(),
                }));
        }
        let errors = validate_site(&site);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("dd-cta cta_link is invalid"))
        );
    }
}
