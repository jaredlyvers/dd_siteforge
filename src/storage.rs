use std::fs;
use std::path::Path;

use anyhow::Context;

use crate::model::Site;

pub fn save_site<P: AsRef<Path>>(path: P, site: &Site) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(site).context("failed to serialize site to JSON")?;
    fs::write(path, json).context("failed to write site JSON")?;
    Ok(())
}

pub fn load_site<P: AsRef<Path>>(path: P) -> anyhow::Result<Site> {
    let json = fs::read_to_string(path).context("failed to read site JSON")?;
    let site = serde_json::from_str(&json).context("failed to parse site JSON")?;
    Ok(site)
}

#[cfg(test)]
mod tests {
    use super::{load_site, save_site};
    use crate::model::{SectionComponent, Site};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn save_and_load_round_trip_preserves_supported_components() {
        let mut site = Site::starter();
        let page = &mut site.pages[0];

        if let crate::model::PageNode::Section(section) = &mut page.nodes[1] {
            section.components.clear();

            section
                .components
                .push(SectionComponent::Banner(crate::model::DdBanner {
                    banner_class: crate::model::BannerClass::BgCenterCenter,
                    banner_data_aos: crate::model::HeroAos::FadeIn,
                    banner_image_url: "/assets/images/banner.jpg".to_string(),
                    banner_image_alt: "Banner A".to_string(),
                }));

            section
                .components
                .push(SectionComponent::Accordion(crate::model::DdAccordion {
                    accordion_type: crate::model::AccordionType::Default,
                    accordion_class: crate::model::AccordionClass::Primary,
                    accordion_aos: crate::model::HeroAos::FadeIn,
                    group_name: "group1".to_string(),
                    items: vec![
                        crate::model::AccordionItem {
                            title: "Acc 1".to_string(),
                            content: "One".to_string(),
                        },
                        crate::model::AccordionItem {
                            title: "Acc 2".to_string(),
                            content: "Two".to_string(),
                        },
                    ],
                    multiple: Some(false),
                }));

            section
                .components
                .push(SectionComponent::Alternating(crate::model::DdAlternating {
                    alternating_type: crate::model::AlternatingType::Default,
                    alternating_class: "-default".to_string(),
                    alternating_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![crate::model::AlternatingItem {
                        image: "/assets/images/alternating.jpg".to_string(),
                        image_alt: "Alt".to_string(),
                        title: "Item A".to_string(),
                        copy: "Copy A".to_string(),
                    }],
                }));

            section.components.swap(0, 2);
        } else {
            panic!("starter site expected section at node index 1");
        }

        let path = unique_temp_path("dd_staticbuilder_storage_roundtrip");
        save_site(&path, &site).expect("save should succeed");
        let loaded = load_site(&path).expect("load should succeed");
        let _ = std::fs::remove_file(&path);

        let loaded_page = &loaded.pages[0];
        let crate::model::PageNode::Section(loaded_section) = &loaded_page.nodes[1] else {
            panic!("loaded page expected section at node index 1");
        };

        assert_eq!(loaded_section.components.len(), 3);

        match &loaded_section.components[0] {
            SectionComponent::Alternating(alternating) => {
                assert_eq!(alternating.items[0].title, "Item A");
            }
            other => panic!("expected alternating at index 0, got {:?}", other),
        }

        match &loaded_section.components[1] {
            SectionComponent::Accordion(acc) => {
                assert_eq!(acc.items.len(), 2);
                assert_eq!(acc.items[0].title, "Acc 1");
            }
            other => panic!("expected accordion at index 1, got {:?}", other),
        }

        match &loaded_section.components[2] {
            SectionComponent::Banner(banner) => {
                assert_eq!(banner.banner_image_url, "/assets/images/banner.jpg");
                assert_eq!(banner.banner_image_alt, "Banner A");
            }
            other => panic!("expected banner at index 2, got {:?}", other),
        }
    }

    #[test]
    fn save_and_load_preserves_nested_reorders_for_supported_collection_components() {
        let mut site = Site::starter();
        let page = &mut site.pages[0];

        if let crate::model::PageNode::Section(section) = &mut page.nodes[1] {
            section.components.clear();

            section
                .components
                .push(SectionComponent::Accordion(crate::model::DdAccordion {
                    accordion_type: crate::model::AccordionType::Default,
                    accordion_class: crate::model::AccordionClass::Primary,
                    accordion_aos: crate::model::HeroAos::FadeIn,
                    group_name: "group1".to_string(),
                    items: vec![
                        crate::model::AccordionItem {
                            title: "Acc 1".to_string(),
                            content: "One".to_string(),
                        },
                        crate::model::AccordionItem {
                            title: "Acc 2".to_string(),
                            content: "Two".to_string(),
                        },
                        crate::model::AccordionItem {
                            title: "Acc 3".to_string(),
                            content: "Three".to_string(),
                        },
                    ],
                    multiple: Some(false),
                }));

            section
                .components
                .push(SectionComponent::Alternating(crate::model::DdAlternating {
                    alternating_type: crate::model::AlternatingType::Default,
                    alternating_class: "-default".to_string(),
                    alternating_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![
                        crate::model::AlternatingItem {
                            image: "/assets/images/a1.jpg".to_string(),
                            image_alt: "A1".to_string(),
                            title: "Alt 1".to_string(),
                            copy: "One".to_string(),
                        },
                        crate::model::AlternatingItem {
                            image: "/assets/images/a2.jpg".to_string(),
                            image_alt: "A2".to_string(),
                            title: "Alt 2".to_string(),
                            copy: "Two".to_string(),
                        },
                    ],
                }));

            if let SectionComponent::Accordion(acc) = &mut section.components[0] {
                acc.items.swap(0, 2);
            }
            if let SectionComponent::Alternating(alt) = &mut section.components[1] {
                alt.items.swap(0, 1);
            }
        } else {
            panic!("starter site expected section at node index 1");
        }

        let path = unique_temp_path("dd_staticbuilder_nested_roundtrip");
        save_site(&path, &site).expect("save should succeed");
        let loaded = load_site(&path).expect("load should succeed");
        let _ = std::fs::remove_file(&path);

        let loaded_page = &loaded.pages[0];
        let crate::model::PageNode::Section(loaded_section) = &loaded_page.nodes[1] else {
            panic!("loaded page expected section at node index 1");
        };

        match &loaded_section.components[0] {
            SectionComponent::Accordion(acc) => {
                assert_eq!(acc.items.len(), 3);
                assert_eq!(acc.items[0].title, "Acc 3");
                assert_eq!(acc.items[1].title, "Acc 2");
                assert_eq!(acc.items[2].title, "Acc 1");
            }
            other => panic!("expected accordion at index 0, got {:?}", other),
        }

        match &loaded_section.components[1] {
            SectionComponent::Alternating(alt) => {
                assert_eq!(alt.items.len(), 2);
                assert_eq!(alt.items[0].title, "Alt 2");
                assert_eq!(alt.items[1].title, "Alt 1");
            }
            other => panic!("expected alternating at index 1, got {:?}", other),
        }
    }

    fn unique_temp_path(prefix: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{now}.json"))
    }
}
