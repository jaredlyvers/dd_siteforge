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
    use crate::model::{SectionComponent, Site, TabItem};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn save_and_load_round_trip_preserves_tui_like_edits() {
        let mut site = Site::starter();
        let page = &mut site.pages[0];

        if let crate::model::PageNode::Section(section) = &mut page.nodes[1] {
            section.components.clear();

            section
                .components
                .push(SectionComponent::Tabs(crate::model::DdTabs {
                    tabs: vec![
                        TabItem {
                            title: "Tab A".to_string(),
                            content: "A content".to_string(),
                        },
                        TabItem {
                            title: "Tab B".to_string(),
                            content: "B content".to_string(),
                        },
                    ],
                    default_tab: Some(0),
                    orientation: Some(crate::model::TabsOrientation::Horizontal),
                }));

            section
                .components
                .push(SectionComponent::Cta(crate::model::DdCta {
                    title: "Old CTA".to_string(),
                    copy: "CTA copy".to_string(),
                    cta_text: "Go".to_string(),
                    cta_link: "/go".to_string(),
                }));

            section
                .components
                .push(SectionComponent::Alert(crate::model::DdAlert {
                    alert_type: crate::model::AlertType::Info,
                    message: "Info".to_string(),
                    title: Some("Initial".to_string()),
                    dismissible: Some(false),
                }));

            if let SectionComponent::Tabs(tabs) = &mut section.components[0] {
                tabs.tabs.swap(0, 1);
                tabs.tabs[0].title = "Tab B Edited".to_string();
            }

            section.components.swap(0, 2);

            if let SectionComponent::Cta(cta) = &mut section.components[1] {
                cta.title = "Updated CTA".to_string();
                cta.cta_link = "/updated".to_string();
            }
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
            SectionComponent::Alert(alert) => assert_eq!(alert.message, "Info"),
            other => panic!("expected alert at index 0, got {:?}", other),
        }

        match &loaded_section.components[1] {
            SectionComponent::Cta(cta) => {
                assert_eq!(cta.title, "Updated CTA");
                assert_eq!(cta.cta_link, "/updated");
            }
            other => panic!("expected cta at index 1, got {:?}", other),
        }

        match &loaded_section.components[2] {
            SectionComponent::Tabs(tabs) => {
                assert_eq!(tabs.tabs.len(), 2);
                assert_eq!(tabs.tabs[0].title, "Tab B Edited");
                assert_eq!(tabs.tabs[1].title, "Tab A");
            }
            other => panic!("expected tabs at index 2, got {:?}", other),
        }
    }

    #[test]
    fn save_and_load_preserves_nested_reorders_for_all_collection_components() {
        let mut site = Site::starter();
        let page = &mut site.pages[0];

        if let crate::model::PageNode::Section(section) = &mut page.nodes[1] {
            section.components.clear();

            section
                .components
                .push(SectionComponent::Accordion(crate::model::DdAccordion {
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
                .push(SectionComponent::Slider(crate::model::DdSlider {
                    slides: vec![
                        crate::model::SlideItem {
                            image: "/assets/images/s1.jpg".to_string(),
                            title: "Slide 1".to_string(),
                            copy: "One".to_string(),
                        },
                        crate::model::SlideItem {
                            image: "/assets/images/s2.jpg".to_string(),
                            title: "Slide 2".to_string(),
                            copy: "Two".to_string(),
                        },
                    ],
                    autoplay: Some(false),
                    speed: Some(400),
                }));

            section
                .components
                .push(SectionComponent::Timeline(crate::model::DdTimeline {
                    events: vec![
                        crate::model::TimelineEvent {
                            date: "2026-01-01".to_string(),
                            title: "Event A".to_string(),
                            description: "A".to_string(),
                        },
                        crate::model::TimelineEvent {
                            date: "2026-01-02".to_string(),
                            title: "Event B".to_string(),
                            description: "B".to_string(),
                        },
                    ],
                }));

            if let SectionComponent::Accordion(acc) = &mut section.components[0] {
                acc.items.swap(0, 2);
            }
            if let SectionComponent::Slider(slider) = &mut section.components[1] {
                slider.slides.swap(0, 1);
            }
            if let SectionComponent::Timeline(tl) = &mut section.components[2] {
                tl.events.swap(0, 1);
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
            SectionComponent::Slider(slider) => {
                assert_eq!(slider.slides.len(), 2);
                assert_eq!(slider.slides[0].title, "Slide 2");
                assert_eq!(slider.slides[1].title, "Slide 1");
            }
            other => panic!("expected slider at index 1, got {:?}", other),
        }

        match &loaded_section.components[2] {
            SectionComponent::Timeline(tl) => {
                assert_eq!(tl.events.len(), 2);
                assert_eq!(tl.events[0].title, "Event B");
                assert_eq!(tl.events[1].title, "Event A");
            }
            other => panic!("expected timeline at index 2, got {:?}", other),
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
