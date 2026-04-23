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
    let site: Site = serde_json::from_str(&json).context("failed to parse site JSON")?;
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
            section.columns[0].components.clear();

            section.columns[0]
                .components
                .push(SectionComponent::Banner(crate::model::DdBanner {
                    parent_class: crate::model::BannerClass::BgCenterCenter,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_image_url: "/assets/images/banner.jpg".to_string(),
                    parent_image_alt: "Banner A".to_string(),
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Accordion(crate::model::DdAccordion {
                    parent_type: crate::model::AccordionType::Default,
                    parent_class: crate::model::AccordionClass::Primary,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_group_name: "group1".to_string(),
                    items: vec![
                        crate::model::AccordionItem {
                            child_title: "Acc 1".to_string(),
                            child_copy: "One".to_string(),
                        },
                        crate::model::AccordionItem {
                            child_title: "Acc 2".to_string(),
                            child_copy: "Two".to_string(),
                        },
                    ],
                    multiple: Some(false),
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Alternating(crate::model::DdAlternating {
                    parent_type: crate::model::AlternatingType::Default,
                    parent_class: "-default".to_string(),
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![crate::model::AlternatingItem {
                        child_image_url: "/assets/images/alternating.jpg".to_string(),
                        child_image_alt: "Alt".to_string(),
                        child_title: "Item A".to_string(),
                        child_copy: "Copy A".to_string(),
                    }],
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Blockquote(crate::model::DdBlockquote {
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_image_url: "/assets/images/blockquote.jpg".to_string(),
                    parent_image_alt: "Person A".to_string(),
                    parent_name: "Person A".to_string(),
                    parent_role: "Title A".to_string(),
                    parent_copy: "Quote A".to_string(),
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Card(crate::model::DdCard {
                    parent_type: crate::model::CardType::Default,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_width: "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24".to_string(),
                    items: vec![crate::model::CardItem {
                        child_image_url: "/assets/images/card.jpg".to_string(),
                        child_image_alt: "Card image".to_string(),
                        child_title: "Card A".to_string(),
                        child_subtitle: "Sub A".to_string(),
                        child_copy: "Copy A".to_string(),
                        child_link_url: Some("/front".to_string()),
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: Some("Learn More".to_string()),
                    }],
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Cta(crate::model::DdCta {
                    parent_class: crate::model::CtaClass::TopLeft,
                    parent_image_url: "/assets/images/cta.jpg".to_string(),
                    parent_image_alt: "CTA image".to_string(),
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_title: "CTA A".to_string(),
                    parent_subtitle: "Sub CTA".to_string(),
                    parent_copy: "Copy CTA".to_string(),
                    parent_link_url: Some("/path".to_string()),
                    parent_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                    parent_link_label: Some("Learn More".to_string()),
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Filmstrip(crate::model::DdFilmstrip {
                    parent_type: crate::model::FilmstripType::Default,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![crate::model::FilmstripItem {
                        child_image_url: "/assets/images/filmstrip-1.jpg".to_string(),
                        child_image_alt: "Filmstrip 1".to_string(),
                        child_title: "Filmstrip Item 1".to_string(),
                    }],
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Milestones(crate::model::DdMilestones {
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_width: "dd-u-1-1 dd-u-md-12-24".to_string(),
                    items: vec![crate::model::MilestonesItem {
                        child_percentage: "70".to_string(),
                        child_title: "Title".to_string(),
                        child_subtitle: "Subtitle".to_string(),
                        child_copy: "Copy".to_string(),
                        child_link_url: Some("/path".to_string()),
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: Some("Learn More".to_string()),
                    }],
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Modal(crate::model::DdModal {
                    parent_title: "Title".to_string(),
                    parent_copy: "Copy".to_string(),
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Slider(crate::model::DdSlider {
                    parent_title: "Slider".to_string(),
                    items: vec![crate::model::SliderItem {
                        child_title: "Title".to_string(),
                        child_copy: "Copy".to_string(),
                        child_link_url: Some("/path".to_string()),
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: Some("Learn More".to_string()),
                        child_image_url: "/assets/images/slider.jpg".to_string(),
                        child_image_alt: "Image alt text".to_string(),
                    }],
                }));

            section.columns[0].components.swap(0, 2);
        } else {
            panic!("starter site expected section at node index 1");
        }

        let path = unique_temp_path("dd_staticsite_storage_roundtrip");
        save_site(&path, &site).expect("save should succeed");
        let loaded = load_site(&path).expect("load should succeed");
        let _ = std::fs::remove_file(&path);

        let loaded_page = &loaded.pages[0];
        let crate::model::PageNode::Section(loaded_section) = &loaded_page.nodes[1] else {
            panic!("loaded page expected section at node index 1");
        };

        assert_eq!(loaded_section.columns[0].components.len(), 10);

        match &loaded_section.columns[0].components[0] {
            SectionComponent::Alternating(alternating) => {
                assert_eq!(alternating.items[0].child_title, "Item A");
            }
            other => panic!("expected alternating at index 0, got {:?}", other),
        }

        match &loaded_section.columns[0].components[1] {
            SectionComponent::Accordion(acc) => {
                assert_eq!(acc.items.len(), 2);
                assert_eq!(acc.items[0].child_title, "Acc 1");
            }
            other => panic!("expected accordion at index 1, got {:?}", other),
        }

        match &loaded_section.columns[0].components[2] {
            SectionComponent::Banner(banner) => {
                assert_eq!(banner.parent_image_url, "/assets/images/banner.jpg");
                assert_eq!(banner.parent_image_alt, "Banner A");
            }
            other => panic!("expected banner at index 2, got {:?}", other),
        }

        match &loaded_section.columns[0].components[3] {
            SectionComponent::Blockquote(blockquote) => {
                assert_eq!(blockquote.parent_name, "Person A");
                assert_eq!(blockquote.parent_copy, "Quote A");
            }
            other => panic!("expected blockquote at index 3, got {:?}", other),
        }

        match &loaded_section.columns[0].components[4] {
            SectionComponent::Card(card) => {
                assert_eq!(card.items.len(), 1);
                assert_eq!(card.items[0].child_title, "Card A");
            }
            other => panic!("expected card at index 4, got {:?}", other),
        }

        match &loaded_section.columns[0].components[5] {
            SectionComponent::Cta(cta) => {
                assert_eq!(cta.parent_title, "CTA A");
                assert_eq!(cta.parent_image_url, "/assets/images/cta.jpg");
            }
            other => panic!("expected cta at index 5, got {:?}", other),
        }

        match &loaded_section.columns[0].components[6] {
            SectionComponent::Filmstrip(filmstrip) => {
                assert_eq!(filmstrip.items.len(), 1);
                assert_eq!(filmstrip.items[0].child_title, "Filmstrip Item 1");
            }
            other => panic!("expected filmstrip at index 6, got {:?}", other),
        }

        match &loaded_section.columns[0].components[7] {
            SectionComponent::Milestones(milestones) => {
                assert_eq!(milestones.items.len(), 1);
                assert_eq!(milestones.items[0].child_title, "Title");
            }
            other => panic!("expected milestones at index 7, got {:?}", other),
        }

        match &loaded_section.columns[0].components[8] {
            SectionComponent::Modal(modal) => {
                assert_eq!(modal.parent_title, "Title");
                assert_eq!(modal.parent_copy, "Copy");
            }
            other => panic!("expected modal at index 8, got {:?}", other),
        }

        match &loaded_section.columns[0].components[9] {
            SectionComponent::Slider(slider) => {
                assert_eq!(slider.parent_title, "Slider");
                assert_eq!(slider.items.len(), 1);
                assert_eq!(slider.items[0].child_title, "Title");
            }
            other => panic!("expected slider at index 9, got {:?}", other),
        }
    }

    #[test]
    fn save_and_load_preserves_nested_reorders_for_supported_collection_components() {
        let mut site = Site::starter();
        let page = &mut site.pages[0];

        if let crate::model::PageNode::Section(section) = &mut page.nodes[1] {
            section.columns[0].components.clear();

            section.columns[0]
                .components
                .push(SectionComponent::Accordion(crate::model::DdAccordion {
                    parent_type: crate::model::AccordionType::Default,
                    parent_class: crate::model::AccordionClass::Primary,
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    parent_group_name: "group1".to_string(),
                    items: vec![
                        crate::model::AccordionItem {
                            child_title: "Acc 1".to_string(),
                            child_copy: "One".to_string(),
                        },
                        crate::model::AccordionItem {
                            child_title: "Acc 2".to_string(),
                            child_copy: "Two".to_string(),
                        },
                        crate::model::AccordionItem {
                            child_title: "Acc 3".to_string(),
                            child_copy: "Three".to_string(),
                        },
                    ],
                    multiple: Some(false),
                }));

            section.columns[0]
                .components
                .push(SectionComponent::Alternating(crate::model::DdAlternating {
                    parent_type: crate::model::AlternatingType::Default,
                    parent_class: "-default".to_string(),
                    parent_data_aos: crate::model::HeroAos::FadeIn,
                    items: vec![
                        crate::model::AlternatingItem {
                            child_image_url: "/assets/images/a1.jpg".to_string(),
                            child_image_alt: "A1".to_string(),
                            child_title: "Alt 1".to_string(),
                            child_copy: "One".to_string(),
                        },
                        crate::model::AlternatingItem {
                            child_image_url: "/assets/images/a2.jpg".to_string(),
                            child_image_alt: "A2".to_string(),
                            child_title: "Alt 2".to_string(),
                            child_copy: "Two".to_string(),
                        },
                    ],
                }));

            if let SectionComponent::Accordion(acc) = &mut section.columns[0].components[0] {
                acc.items.swap(0, 2);
            }
            if let SectionComponent::Alternating(alt) = &mut section.columns[0].components[1] {
                alt.items.swap(0, 1);
            }
        } else {
            panic!("starter site expected section at node index 1");
        }

        let path = unique_temp_path("dd_staticsite_nested_roundtrip");
        save_site(&path, &site).expect("save should succeed");
        let loaded = load_site(&path).expect("load should succeed");
        let _ = std::fs::remove_file(&path);

        let loaded_page = &loaded.pages[0];
        let crate::model::PageNode::Section(loaded_section) = &loaded_page.nodes[1] else {
            panic!("loaded page expected section at node index 1");
        };

        match &loaded_section.columns[0].components[0] {
            SectionComponent::Accordion(acc) => {
                assert_eq!(acc.items.len(), 3);
                assert_eq!(acc.items[0].child_title, "Acc 3");
                assert_eq!(acc.items[1].child_title, "Acc 2");
                assert_eq!(acc.items[2].child_title, "Acc 1");
            }
            other => panic!("expected accordion at index 0, got {:?}", other),
        }

        match &loaded_section.columns[0].components[1] {
            SectionComponent::Alternating(alt) => {
                assert_eq!(alt.items.len(), 2);
                assert_eq!(alt.items[0].child_title, "Alt 2");
                assert_eq!(alt.items[1].child_title, "Alt 1");
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

    #[test]
    fn slug_locked_defaults_to_false_on_load_of_legacy_site_json() {
        // Legacy JSON that predates slug_locked — must still load.
        let json = r##"{
          "schema_version": 1,
          "id": "s",
          "name": "n",
          "theme": {"primary_color":"#000","secondary_color":"#000","tertiary_color":"#000","support_color":"#000"},
          "header": {"id":"h","custom_css":null,"alert":null,"sections":[]},
          "footer": {"id":"f","custom_css":null,"sections":[]},
          "pages": [{
            "id":"p1","slug":"index",
            "head":{"title":"Home","meta_description":null,"canonical_url":null,
                    "robots":"index, follow","schema_type":"WebPage",
                    "og_title":null,"og_description":null,"og_image":null},
            "nodes":[]
          }]
        }"##;
        let site: crate::model::Site = serde_json::from_str(json).expect("legacy JSON should load");
        assert!(!site.pages[0].slug_locked, "legacy pages load with slug_locked = false");
    }

    #[test]
    fn slug_locked_round_trips_through_save_and_load() {
        let tmp = unique_temp_path("dd_site_slug_lock");
        let mut site = crate::model::Site::starter();
        site.pages[0].slug_locked = true;
        save_site(&tmp, &site).expect("save ok");
        let loaded = load_site(&tmp).expect("load ok");
        std::fs::remove_file(&tmp).ok();
        assert!(loaded.pages[0].slug_locked);
    }

    #[test]
    fn export_dir_defaults_to_none_on_legacy_json() {
        // Legacy JSON missing export_dir — must still load.
        let json = r##"{
          "schema_version": 1,
          "id": "s",
          "name": "n",
          "theme": {"primary_color":"#000","secondary_color":"#000","tertiary_color":"#000","support_color":"#000"},
          "header": {"id":"h","custom_css":null,"alert":null,"sections":[]},
          "footer": {"id":"f","custom_css":null,"sections":[]},
          "pages": []
        }"##;
        let site: crate::model::Site = serde_json::from_str(json).expect("legacy JSON should load");
        assert!(site.export_dir.is_none(), "legacy sites load with export_dir = None");
    }

    #[test]
    fn export_dir_round_trips_through_save_and_load() {
        let tmp = unique_temp_path("dd_site_export_dir_roundtrip");
        let mut site = crate::model::Site::starter();
        site.export_dir = Some("./web/".to_string());
        save_site(&tmp, &site).expect("save ok");
        let loaded = load_site(&tmp).expect("load ok");
        std::fs::remove_file(&tmp).ok();
        assert_eq!(loaded.export_dir.as_deref(), Some("./web/"));
    }
}
