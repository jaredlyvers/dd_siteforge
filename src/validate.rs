use crate::model::{DdSection, NavigationItem, NavigationKind, PageNode, SectionComponent, Site};

pub fn validate_site(site: &Site) -> Vec<String> {
    let mut errors = Vec::new();
    let mut slugs = std::collections::HashSet::new();

    if site.pages.is_empty() {
        errors.push("Site must include at least one page.".to_string());
    }

    validate_header(&site.header, &mut errors);
    validate_footer(&site.footer, &mut errors);

    for page in &site.pages {
        if page.head.title.trim().is_empty() {
            errors.push(format!("Page '{}' is missing a head title.", page.id));
        }
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
                    if hero.parent_title.trim().is_empty() {
                        errors.push(format!("Page '{}' hero is missing parent_title.", page.id));
                    }
                    if !hero.parent_image_url.trim().is_empty()
                        && hero
                            .parent_image_alt
                            .as_deref()
                            .unwrap_or("")
                            .trim()
                            .is_empty()
                    {
                        errors.push(format!(
                            "Page '{}' hero image is missing parent_image_alt text.",
                            page.id
                        ));
                    }
                    if hero.link_1_label.is_some() ^ hero.link_1_url.is_some() {
                        errors.push(format!(
                            "Page '{}' hero must provide both link_1_label and link_1_url together.",
                            page.id
                        ));
                    }
                    if let Some(link) = &hero.link_1_url {
                        if !is_valid_url(link) {
                            errors.push(format!("Page '{}' hero link_1_url is invalid.", page.id));
                        }
                    }
                    if hero.link_2_label.is_some() ^ hero.link_2_url.is_some() {
                        errors.push(format!(
                            "Page '{}' hero must provide both link_2_label and link_2_url together.",
                            page.id
                        ));
                    }
                    if let Some(link) = &hero.link_2_url {
                        if !is_valid_url(link) {
                            errors.push(format!("Page '{}' hero link_2_url is invalid.", page.id));
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
                    if section.columns.is_empty() {
                        errors.push(format!("Section '{}' has no columns.", section.id));
                    }
                    let mut column_ids = std::collections::HashSet::new();
                    for column in &section.columns {
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
                if item.child_image_url.trim().is_empty()
                    || item.child_image_alt.trim().is_empty()
                    || item.child_title.trim().is_empty()
                    || item.child_copy.trim().is_empty()
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
            if card.parent_width.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-card with empty parent_width.",
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
                if item.child_image_url.trim().is_empty()
                    || item.child_image_alt.trim().is_empty()
                    || item.child_title.trim().is_empty()
                    || item.child_subtitle.trim().is_empty()
                    || item.child_copy.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} has missing required fields.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if !is_valid_url(&item.child_image_url) {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} child_image_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                let has_link_url = item
                    .child_link_url
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                let has_link_label = item
                    .child_link_label
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                if has_link_url ^ has_link_label {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} must provide both child_link_url and child_link_label together.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if let Some(url) = item.child_link_url.as_deref()
                    && !url.trim().is_empty()
                    && !is_valid_url(url)
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-card item {} child_link_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Banner(banner) => {
            if banner.parent_image_url.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-banner with empty parent_image_url.",
                    page_id, section_id
                ));
            }
            if banner.parent_image_alt.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-banner with empty parent_image_alt.",
                    page_id, section_id
                ));
            }
            if !is_valid_url(&banner.parent_image_url) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-banner parent_image_url is invalid.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Cta(cta) => {
            if cta.parent_image_url.trim().is_empty()
                || cta.parent_image_alt.trim().is_empty()
                || cta.parent_title.trim().is_empty()
                || cta.parent_subtitle.trim().is_empty()
                || cta.parent_copy.trim().is_empty()
            {
                errors.push(format!(
                    "Page '{}' section '{}' dd-cta has missing required fields.",
                    page_id, section_id
                ));
            }
            if !is_valid_url(&cta.parent_image_url) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-cta parent_image_url is invalid.",
                    page_id, section_id
                ));
            }
            let has_link_url = cta
                .parent_link_url
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty());
            let has_link_label = cta
                .parent_link_label
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty());
            if has_link_url ^ has_link_label {
                errors.push(format!(
                    "Page '{}' section '{}' dd-cta must provide both parent_link_url and parent_link_label together.",
                    page_id, section_id
                ));
            }
            if let Some(url) = cta.parent_link_url.as_deref()
                && !url.trim().is_empty()
                && !is_valid_url(url)
            {
                errors.push(format!(
                    "Page '{}' section '{}' dd-cta parent_link_url is invalid.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Filmstrip(filmstrip) => {
            if filmstrip.items.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-filmstrip with no items.",
                    page_id, section_id
                ));
            }
            for (idx, item) in filmstrip.items.iter().enumerate() {
                if item.child_image_url.trim().is_empty()
                    || item.child_image_alt.trim().is_empty()
                    || item.child_title.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-filmstrip item {} has missing required fields.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if !is_valid_url(&item.child_image_url) {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-filmstrip item {} child_image_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Milestones(milestones) => {
            if milestones.parent_width.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-milestones with empty parent_width.",
                    page_id, section_id
                ));
            }
            if milestones.items.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-milestones with no items.",
                    page_id, section_id
                ));
            }
            for (idx, item) in milestones.items.iter().enumerate() {
                if item.child_percentage.trim().is_empty()
                    || item.child_title.trim().is_empty()
                    || item.child_subtitle.trim().is_empty()
                    || item.child_copy.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-milestones item {} has missing required fields.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                let has_link_url = item
                    .child_link_url
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                let has_link_label = item
                    .child_link_label
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                if has_link_url ^ has_link_label {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-milestones item {} must provide both child_link_url and child_link_label together.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if let Some(url) = item.child_link_url.as_deref()
                    && !url.trim().is_empty()
                    && !is_valid_url(url)
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-milestones item {} child_link_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Modal(modal) => {
            if modal.parent_title.trim().is_empty() || modal.parent_copy.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-modal has missing required fields.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Slider(slider) => {
            if slider.items.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has dd-slider with no items.",
                    page_id, section_id
                ));
            }
            for (idx, item) in slider.items.iter().enumerate() {
                if item.child_title.trim().is_empty()
                    || item.child_copy.trim().is_empty()
                    || item.child_image_url.trim().is_empty()
                    || item.child_image_alt.trim().is_empty()
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-slider item {} has missing required fields.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if !is_valid_url(&item.child_image_url) {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-slider item {} child_image_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                let has_link_url = item
                    .child_link_url
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                let has_link_label = item
                    .child_link_label
                    .as_deref()
                    .is_some_and(|v| !v.trim().is_empty());
                if has_link_url ^ has_link_label {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-slider item {} must provide both child_link_url and child_link_label together.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
                if let Some(url) = item.child_link_url.as_deref()
                    && !url.trim().is_empty()
                    && !is_valid_url(url)
                {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-slider item {} child_link_url is invalid.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Accordion(accordion) => {
            if accordion.parent_group_name.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-accordion missing parent_group_name.",
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
                if item.child_title.trim().is_empty() || item.child_copy.trim().is_empty() {
                    errors.push(format!(
                        "Page '{}' section '{}' dd-accordion item {} has missing child_title/child_copy.",
                        page_id,
                        section_id,
                        idx + 1
                    ));
                }
            }
        }
        SectionComponent::Blockquote(blockquote) => {
            if blockquote.parent_image_url.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' has a dd-blockquote with empty parent_image_url.",
                    page_id, section_id
                ));
            }
            if blockquote.parent_image_alt.trim().is_empty()
                || blockquote.parent_name.trim().is_empty()
                || blockquote.parent_role.trim().is_empty()
                || blockquote.parent_copy.trim().is_empty()
            {
                errors.push(format!(
                    "Page '{}' section '{}' dd-blockquote has missing required fields.",
                    page_id, section_id
                ));
            }
            if !is_valid_url(&blockquote.parent_image_url) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-blockquote parent_image_url is invalid.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Alert(alert) => {
            if alert.parent_copy.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-alert has missing required parent_copy.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Image(image) => {
            if image.parent_image_url.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-image is missing parent_image_url.",
                    page_id, section_id
                ));
            } else if !is_valid_url(&image.parent_image_url) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-image parent_image_url is invalid.",
                    page_id, section_id
                ));
            }
            if image.parent_image_alt.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-image is missing parent_image_alt.",
                    page_id, section_id
                ));
            }
            if let Some(url) = image.parent_link_url.as_deref()
                && !url.trim().is_empty()
                && !is_valid_url(url)
            {
                errors.push(format!(
                    "Page '{}' section '{}' dd-image parent_link_url is invalid.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::RichText(rt) => {
            if rt.parent_copy.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-rich_text is missing parent_copy.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::Navigation(nav) => {
            if nav.items.is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-navigation has no items.",
                    page_id, section_id
                ));
            }
            for (idx, item) in nav.items.iter().enumerate() {
                validate_navigation_item(
                    item,
                    &format!("{}", idx + 1),
                    page_id,
                    section_id,
                    errors,
                );
            }
        }
        SectionComponent::HeaderSearch(search) => {
            if search.parent_width.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-header-search is missing parent_width.",
                    page_id, section_id
                ));
            }
        }
        SectionComponent::HeaderMenu(menu) => {
            if menu.parent_width.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-header-menu is missing parent_width.",
                    page_id, section_id
                ));
            }
        }
    }
}

fn validate_navigation_item(
    item: &NavigationItem,
    path: &str,
    page_id: &str,
    section_id: &str,
    errors: &mut Vec<String>,
) {
    if item.child_link_label.trim().is_empty() {
        errors.push(format!(
            "Page '{}' section '{}' dd-navigation item {} missing child_link_label.",
            page_id, section_id, path
        ));
    }
    match item.child_kind {
        NavigationKind::Link => {
            let url = item.child_link_url.as_deref().unwrap_or("");
            if url.trim().is_empty() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-navigation item {} kind=link requires child_link_url.",
                    page_id, section_id, path
                ));
            } else if !is_valid_url(url) {
                errors.push(format!(
                    "Page '{}' section '{}' dd-navigation item {} child_link_url is invalid.",
                    page_id, section_id, path
                ));
            }
        }
        NavigationKind::Button => {
            if item
                .child_link_url
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty())
            {
                errors.push(format!(
                    "Page '{}' section '{}' dd-navigation item {} kind=button must not provide child_link_url.",
                    page_id, section_id, path
                ));
            }
            if item.child_link_target.is_some() {
                errors.push(format!(
                    "Page '{}' section '{}' dd-navigation item {} kind=button must not provide child_link_target.",
                    page_id, section_id, path
                ));
            }
        }
    }
    for (idx, child) in item.items.iter().enumerate() {
        validate_navigation_item(
            child,
            &format!("{}.{}", path, idx + 1),
            page_id,
            section_id,
            errors,
        );
    }
}

fn validate_header(header: &crate::model::DdHeader, errors: &mut Vec<String>) {
    if header.id.trim().is_empty() {
        errors.push("site.header has empty id.".to_string());
    }
    if header.sections.is_empty() {
        errors.push("site.header must have at least one section.".to_string());
    }
    for section in &header.sections {
        validate_section_context(
            section,
            "header",
            &[
                "dd-image",
                "dd-rich_text",
                "dd-navigation",
                "dd-header-search",
                "dd-header-menu",
            ],
            errors,
        );
    }
}

fn validate_footer(footer: &crate::model::DdFooter, errors: &mut Vec<String>) {
    if footer.id.trim().is_empty() {
        errors.push("site.footer has empty id.".to_string());
    }
    if footer.sections.is_empty() {
        errors.push("site.footer must have at least one section.".to_string());
    }
    for section in &footer.sections {
        validate_section_context(
            section,
            "footer",
            &["dd-image", "dd-rich_text", "dd-navigation"],
            errors,
        );
    }
}

fn validate_section_context(
    section: &DdSection,
    scope: &str,
    allowed_types: &[&str],
    errors: &mut Vec<String>,
) {
    for column in &section.columns {
        for component in &column.components {
            let ty = section_component_type_name(component);
            if !allowed_types.contains(&ty) {
                errors.push(format!(
                    "site.{} section '{}' column '{}' contains disallowed component type '{}'; allowed: {:?}",
                    scope, section.id, column.id, ty, allowed_types
                ));
            }
        }
    }
}

fn section_component_type_name(component: &SectionComponent) -> &'static str {
    match component {
        SectionComponent::Alternating(_) => "dd-alternating",
        SectionComponent::Card(_) => "dd-card",
        SectionComponent::Cta(_) => "dd-cta",
        SectionComponent::Filmstrip(_) => "dd-filmstrip",
        SectionComponent::Milestones(_) => "dd-milestones",
        SectionComponent::Slider(_) => "dd-slider",
        SectionComponent::Modal(_) => "dd-modal",
        SectionComponent::Banner(_) => "dd-banner",
        SectionComponent::Accordion(_) => "dd-accordion",
        SectionComponent::Blockquote(_) => "dd-blockquote",
        SectionComponent::Alert(_) => "dd-alert",
        SectionComponent::Image(_) => "dd-image",
        SectionComponent::RichText(_) => "dd-rich_text",
        SectionComponent::Navigation(_) => "dd-navigation",
        SectionComponent::HeaderSearch(_) => "dd-header-search",
        SectionComponent::HeaderMenu(_) => "dd-header-menu",
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

pub fn validate_site_with_root(
    site: &Site,
    root: Option<&std::path::Path>,
) -> Vec<String> {
    let mut errors = validate_site(site);
    let Some(root) = root else {
        return errors;
    };
    for page in &site.pages {
        let refs = collect_image_refs(page);
        for (label, value) in refs {
            check_local_image(root, &label, &value, &mut errors);
        }
    }
    errors
}

fn check_local_image(
    root: &std::path::Path,
    label: &str,
    value: &str,
    errors: &mut Vec<String>,
) {
    let prefix = "assets/images/";
    let v = value.trim_start_matches('/');
    let Some(rest) = v.strip_prefix(prefix) else {
        return;
    };
    let resolved = root.join("source").join("images").join(rest);
    if !resolved.exists() {
        errors.push(format!(
            "Missing local image: {} → {} (expected at source/images/{})",
            label, value, rest
        ));
    }
}

fn collect_image_refs(page: &crate::model::Page) -> Vec<(String, String)> {
    let mut refs: Vec<(String, String)> = Vec::new();
    for node in &page.nodes {
        match node {
            crate::model::PageNode::Hero(hero) => {
                refs.push((
                    format!("page '{}' hero parent_image_url", page.id),
                    hero.parent_image_url.clone(),
                ));
                if let Some(s) = hero.parent_image_mobile.as_deref() {
                    refs.push((
                        format!("page '{}' hero parent_image_mobile", page.id),
                        s.to_string(),
                    ));
                }
                if let Some(s) = hero.parent_image_tablet.as_deref() {
                    refs.push((
                        format!("page '{}' hero parent_image_tablet", page.id),
                        s.to_string(),
                    ));
                }
                if let Some(s) = hero.parent_image_desktop.as_deref() {
                    refs.push((
                        format!("page '{}' hero parent_image_desktop", page.id),
                        s.to_string(),
                    ));
                }
            }
            crate::model::PageNode::Section(section) => {
                for col in &section.columns {
                    for comp in &col.components {
                        collect_component_image_refs(page, comp, &mut refs);
                    }
                }
            }
        }
    }
    refs
}

fn collect_component_image_refs(
    page: &crate::model::Page,
    comp: &crate::model::SectionComponent,
    refs: &mut Vec<(String, String)>,
) {
    use crate::model::SectionComponent::*;
    let lbl = |suffix: &str| format!("page '{}' {}", page.id, suffix);
    match comp {
        Banner(b) => refs.push((lbl("banner image"), b.parent_image_url.clone())),
        Cta(c) => refs.push((lbl("cta image"), c.parent_image_url.clone())),
        Image(i) => refs.push((lbl("image"), i.parent_image_url.clone())),
        Blockquote(b) => refs.push((lbl("blockquote image"), b.parent_image_url.clone())),
        Card(c) => {
            for (n, item) in c.items.iter().enumerate() {
                refs.push((lbl(&format!("card item {} image", n + 1)), item.child_image_url.clone()));
            }
        }
        Filmstrip(f) => {
            for (n, item) in f.items.iter().enumerate() {
                refs.push((lbl(&format!("filmstrip item {} image", n + 1)), item.child_image_url.clone()));
            }
        }
        Slider(s) => {
            for (n, item) in s.items.iter().enumerate() {
                refs.push((lbl(&format!("slider item {} image", n + 1)), item.child_image_url.clone()));
            }
        }
        Alternating(a) => {
            for (n, item) in a.items.iter().enumerate() {
                refs.push((lbl(&format!("alternating item {} image", n + 1)), item.child_image_url.clone()));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_site, validate_site_with_root};
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
            hero.parent_title.clear();
            hero.parent_subtitle.clear();
            hero.parent_image_url.clear();
        }
        let errors = validate_site(&site);
        assert!(errors.iter().any(|e| e.contains("missing parent_title")));
        assert!(!errors.iter().any(|e| e.contains("missing parent_subtitle")));
        assert!(!errors
            .iter()
            .any(|e| e.contains("missing parent_image_url")));
    }

    #[test]
    fn detects_duplicate_page_slug() {
        let mut site = Site::starter();
        site.pages.push(site.pages[0].clone());
        let errors = validate_site(&site);
        assert!(errors.iter().any(|e| e.contains("Duplicate page slug")));
    }

    #[test]
    fn validate_with_root_flags_missing_local_image() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_missing_img_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut site = Site::starter();
        if let PageNode::Hero(hero) = &mut site.pages[0].nodes[0] {
            hero.parent_image_url = "/assets/images/missing.jpg".to_string();
            hero.parent_image_alt = Some("alt".to_string());
        }
        let errors = validate_site_with_root(&site, Some(&tmp));
        assert!(
            errors.iter().any(|e| e.contains("Missing local image")),
            "expected missing-image error, got: {:?}",
            errors
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn validate_with_root_passes_when_image_exists() {
        let tmp = std::env::temp_dir().join(format!(
            "dd_present_img_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let imgs = tmp.join("source").join("images");
        std::fs::create_dir_all(&imgs).unwrap();
        std::fs::write(imgs.join("hero.jpg"), b"fake").unwrap();

        let mut site = Site::starter();
        if let PageNode::Hero(hero) = &mut site.pages[0].nodes[0] {
            hero.parent_image_url = "assets/images/hero.jpg".to_string();
            hero.parent_image_alt = Some("alt".to_string());
        }
        let errors = validate_site_with_root(&site, Some(&tmp));
        assert!(
            errors.iter().all(|e| !e.contains("Missing local image")),
            "no missing-image error expected, got: {:?}",
            errors
        );
        std::fs::remove_dir_all(&tmp).ok();
    }
}
