//! Enum cycle helpers and component labels.

pub(in crate::tui) fn section_class_to_str(v: crate::model::SectionClass) -> &'static str {
    match v {
        crate::model::SectionClass::Contained => "-contained",
        crate::model::SectionClass::ContainedMd => "-contained-md",
        crate::model::SectionClass::ContainedLg => "-contained-lg",
        crate::model::SectionClass::ContainedXl => "-contained-xl",
        crate::model::SectionClass::ContainedXxl => "-contained-xxl",
        crate::model::SectionClass::FullFull => "-full-full",
        crate::model::SectionClass::FullContained => "-full-contained",
        crate::model::SectionClass::FullContainedMd => "-full-contained-md",
        crate::model::SectionClass::FullContainedLg => "-full-contained-lg",
        crate::model::SectionClass::FullContainedXl => "-full-contained-xl",
        crate::model::SectionClass::FullContainedXxl => "-full-contained-xxl",
    }
}

#[allow(dead_code)]
pub(in crate::tui) fn next_alert_type(current: crate::model::AlertType, forward: bool) -> crate::model::AlertType {
    use crate::model::AlertType;
    let all = [
        AlertType::Default,
        AlertType::Info,
        AlertType::Warning,
        AlertType::Error,
        AlertType::Success,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

#[allow(dead_code)]
pub(in crate::tui) fn next_alert_class(current: crate::model::AlertClass, forward: bool) -> crate::model::AlertClass {
    use crate::model::AlertClass;
    let all = [AlertClass::Default, AlertClass::Compact];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next_idx = if forward {
        (idx + 1) % all.len()
    } else if idx == 0 {
        all.len() - 1
    } else {
        idx - 1
    };
    all[next_idx]
}

pub(in crate::tui) fn input_lines_preserve(s: &str) -> Vec<String> {
    s.split('\n').map(|line| line.to_string()).collect()
}

pub(in crate::tui) fn cursor_from_row_col(lines: &[String], target_row: usize, target_col: usize) -> usize {
    let row = target_row.min(lines.len().saturating_sub(1));
    let mut cursor = 0usize;
    for line in lines.iter().take(row) {
        cursor += line.chars().count() + 1;
    }
    let line_len = lines.get(row).map(|line| line.chars().count()).unwrap_or(0);
    cursor + target_col.min(line_len)
}

pub(in crate::tui) fn component_label(component: &crate::model::SectionComponent) -> &'static str {
    match component {
        crate::model::SectionComponent::Cta(_) => "dd-cta",
        crate::model::SectionComponent::Filmstrip(_) => "dd-filmstrip",
        crate::model::SectionComponent::Milestones(_) => "dd-milestones",
        crate::model::SectionComponent::Slider(_) => "dd-slider",
        crate::model::SectionComponent::Modal(_) => "dd-modal",
        crate::model::SectionComponent::Banner(_) => "dd-banner",
        crate::model::SectionComponent::Card(_) => "dd-card",
        crate::model::SectionComponent::Blockquote(_) => "dd-blockquote",
        crate::model::SectionComponent::Accordion(_) => "dd-accordion",
        crate::model::SectionComponent::Alternating(_) => "dd-alternating",
        crate::model::SectionComponent::Alert(_) => "dd-alert",
        crate::model::SectionComponent::Image(_) => "dd-image",
        crate::model::SectionComponent::RichText(_) => "dd-rich_text",
        crate::model::SectionComponent::Navigation(_) => "dd-navigation",
        crate::model::SectionComponent::HeaderSearch(_) => "dd-header-search",
        crate::model::SectionComponent::HeaderMenu(_) => "dd-header-menu",
    }
}

pub(in crate::tui) fn component_blueprint_label(component: &crate::model::SectionComponent) -> String {
    match component {
        crate::model::SectionComponent::Cta(v) => {
            format!("dd-cta | parent_title: {}", v.parent_title)
        }
        crate::model::SectionComponent::Filmstrip(v) => format!(
            "dd-filmstrip | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Milestones(v) => format!(
            "dd-milestones | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Slider(v) => format!(
            "dd-slider | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Modal(v) => {
            format!("dd-modal | parent_title: {}", v.parent_title)
        }
        crate::model::SectionComponent::Accordion(v) => format!(
            "dd-accordion | accordion_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Alternating(v) => format!(
            "dd-alternating | alternating_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Card(v) => format!(
            "dd-card | child_title: {}",
            v.items
                .first()
                .map(|i| i.child_title.as_str())
                .unwrap_or("(none)")
        ),
        crate::model::SectionComponent::Blockquote(v) => format!(
            "dd-blockquote | parent_name: {} | parent_role: {}",
            v.parent_name, v.parent_role
        ),
        _ => component_label(component).to_string(),
    }
}

pub(in crate::tui) fn hero_image_class_to_str(v: crate::model::HeroImageClass) -> &'static str {
    match v {
        crate::model::HeroImageClass::Contained => "-contained",
        crate::model::HeroImageClass::ContainedMd => "-contained-md",
        crate::model::HeroImageClass::ContainedLg => "-contained-lg",
        crate::model::HeroImageClass::ContainedXl => "-contained-xl",
        crate::model::HeroImageClass::ContainedXxl => "-contained-xxl",
        crate::model::HeroImageClass::FullFull => "-full-full",
        crate::model::HeroImageClass::FullContained => "-full-contained",
        crate::model::HeroImageClass::FullContainedMd => "-full-contained-md",
        crate::model::HeroImageClass::FullContainedLg => "-full-contained-lg",
        crate::model::HeroImageClass::FullContainedXl => "-full-contained-xl",
        crate::model::HeroImageClass::FullContainedXxl => "-full-contained-xxl",
    }
}

pub(in crate::tui) fn parent_data_aos_to_str(v: crate::model::HeroAos) -> &'static str {
    match v {
        crate::model::HeroAos::FadeIn => "fade-in",
        crate::model::HeroAos::FadeUp => "fade-up",
        crate::model::HeroAos::FadeRight => "fade-right",
        crate::model::HeroAos::FadeDown => "fade-down",
        crate::model::HeroAos::FadeLeft => "fade-left",
        crate::model::HeroAos::ZoomIn => "zoom-in",
        crate::model::HeroAos::ZoomInUp => "zoom-in-up",
        crate::model::HeroAos::ZoomInDown => "zoom-in-down",
    }
}

#[allow(dead_code)]
pub(in crate::tui) fn next_navigation_type(
    current: crate::model::NavigationType,
    forward: bool,
) -> crate::model::NavigationType {
    use crate::model::NavigationType;
    let all = [NavigationType::HeaderNav, NavigationType::FooterNav];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

#[allow(dead_code)]
pub(in crate::tui) fn next_navigation_class(
    current: crate::model::NavigationClass,
    forward: bool,
) -> crate::model::NavigationClass {
    use crate::model::NavigationClass;
    let all = [
        NavigationClass::MainMenu,
        NavigationClass::MenuSecondary,
        NavigationClass::MenuTertiary,
        NavigationClass::FooterMenu,
        NavigationClass::FooterMenuSecondary,
        NavigationClass::FooterMenuTertiary,
        NavigationClass::SocialMenu,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

#[allow(dead_code)]
pub(in crate::tui) fn navigation_kind_to_str(v: crate::model::NavigationKind) -> &'static str {
    match v {
        crate::model::NavigationKind::Link => "link",
        crate::model::NavigationKind::Button => "button",
    }
}

#[allow(dead_code)]
pub(in crate::tui) fn parse_navigation_kind(raw: &str) -> Option<crate::model::NavigationKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "link" => Some(crate::model::NavigationKind::Link),
        "button" => Some(crate::model::NavigationKind::Button),
        _ => None,
    }
}

#[allow(dead_code)]
pub(in crate::tui) fn next_navigation_kind(
    current: crate::model::NavigationKind,
    forward: bool,
) -> crate::model::NavigationKind {
    let _ = forward;
    match current {
        crate::model::NavigationKind::Link => crate::model::NavigationKind::Button,
        crate::model::NavigationKind::Button => crate::model::NavigationKind::Link,
    }
}

#[allow(dead_code)]
pub(in crate::tui) fn robots_directive_to_str(v: crate::model::RobotsDirective) -> &'static str {
    match v {
        crate::model::RobotsDirective::IndexFollow => "index, follow",
        crate::model::RobotsDirective::NoindexFollow => "noindex, follow",
        crate::model::RobotsDirective::IndexNofollow => "index, nofollow",
        crate::model::RobotsDirective::NoindexNofollow => "noindex, nofollow",
    }
}

#[allow(dead_code)]
pub(in crate::tui) fn next_robots_directive(
    current: crate::model::RobotsDirective,
    forward: bool,
) -> crate::model::RobotsDirective {
    use crate::model::RobotsDirective;
    let all = [
        RobotsDirective::IndexFollow,
        RobotsDirective::NoindexFollow,
        RobotsDirective::IndexNofollow,
        RobotsDirective::NoindexNofollow,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}

#[allow(dead_code)]
pub(in crate::tui) fn schema_type_to_str(v: crate::model::SchemaType) -> &'static str {
    match v {
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

#[allow(dead_code)]
pub(in crate::tui) fn next_schema_type(
    current: crate::model::SchemaType,
    forward: bool,
) -> crate::model::SchemaType {
    use crate::model::SchemaType;
    let all = [
        SchemaType::WebPage,
        SchemaType::Article,
        SchemaType::AboutPage,
        SchemaType::ContactPage,
        SchemaType::CollectionPage,
        SchemaType::Organization,
        SchemaType::LocalBusiness,
        SchemaType::Product,
        SchemaType::Service,
    ];
    let idx = all.iter().position(|v| *v == current).unwrap_or(0);
    let next = if forward {
        (idx + 1) % all.len()
    } else {
        (idx + all.len() - 1) % all.len()
    };
    all[next]
}
