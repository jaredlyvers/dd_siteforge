//! Insert-picker component kinds.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ComponentKind {
    Hero,
    Section,
    Banner,
    Cta,
    Blockquote,
    Accordion,
    Alternating,
    Card,
    Filmstrip,
    Milestones,
    Modal,
    Slider,
    Alert,
    Image,
    RichText,
    Navigation,
    HeaderSearch,
    HeaderMenu,
}



impl ComponentKind {
    pub(super) fn all() -> &'static [Self] {
        &[
            Self::Hero,
            Self::Section,
            Self::Cta,
            Self::Banner,
            Self::Blockquote,
            Self::Accordion,
            Self::Alternating,
            Self::Card,
            Self::Filmstrip,
            Self::Milestones,
            Self::Modal,
            Self::Slider,
            Self::Alert,
            Self::Image,
            Self::RichText,
            Self::Navigation,
            Self::HeaderSearch,
            Self::HeaderMenu,
        ]
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            ComponentKind::Hero => "dd-hero",
            ComponentKind::Section => "dd-section",
            ComponentKind::Cta => "dd-cta",
            ComponentKind::Banner => "dd-banner",
            ComponentKind::Blockquote => "dd-blockquote",
            ComponentKind::Accordion => "dd-accordion",
            ComponentKind::Alternating => "dd-alternating",
            ComponentKind::Card => "dd-card",
            ComponentKind::Filmstrip => "dd-filmstrip",
            ComponentKind::Milestones => "dd-milestones",
            ComponentKind::Modal => "dd-modal",
            ComponentKind::Slider => "dd-slider",
            ComponentKind::Alert => "dd-alert",
            ComponentKind::Image => "dd-image",
            ComponentKind::RichText => "dd-rich_text",
            ComponentKind::Navigation => "dd-navigation",
            ComponentKind::HeaderSearch => "dd-header-search",
            ComponentKind::HeaderMenu => "dd-header-menu",
        }
    }

    pub(super) fn default_component(self) -> crate::model::SectionComponent {
        match self {
            ComponentKind::Hero | ComponentKind::Section => {
                unreachable!("top-level kinds do not map to section components")
            }
            ComponentKind::Cta => crate::model::SectionComponent::Cta(crate::model::DdCta {
                parent_class: crate::model::CtaClass::TopLeft,
                parent_image_url: "https://dummyimage.com/1920x1080/000000/fff".to_string(),
                parent_image_alt: "Image alt".to_string(),
                sal: crate::model::SalAnimation::Fade,
                parent_title: "Title".to_string(),
                parent_subtitle: "Subtitle".to_string(),
                parent_copy: "Copy".to_string(),
                parent_link_url: Some("/path".to_string()),
                parent_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                parent_link_label: Some("Learn More".to_string()),
            }),
            ComponentKind::Banner => {
                crate::model::SectionComponent::Banner(crate::model::DdBanner {
                    parent_class: crate::model::BannerClass::BgCenterCenter,
                    sal: crate::model::SalAnimation::Fade,
                    parent_image_url: "https://dummyimage.com/1920x1080/000/fff".to_string(),
                    parent_image_alt: "Banner alt text".to_string(),
                })
            }
            ComponentKind::Blockquote => {
                crate::model::SectionComponent::Blockquote(crate::model::DdBlockquote {
                    sal: crate::model::SalAnimation::Fade,
                    parent_image_url: "https://dummyimage.com/512x512/000/fff".to_string(),
                    parent_image_alt: "blockquote Persons Name".to_string(),
                    parent_name: "blockquote Persons Name".to_string(),
                    parent_role: "blockquote Persons Title".to_string(),
                    parent_copy: "blockquote content".to_string(),
                })
            }
            ComponentKind::Accordion => {
                crate::model::SectionComponent::Accordion(crate::model::DdAccordion {
                    parent_type: crate::model::AccordionType::Default,
                    parent_class: crate::model::AccordionClass::Primary,
                    sal: crate::model::SalAnimation::Fade,
                    parent_group_name: "group1".to_string(),
                    items: vec![crate::model::AccordionItem {
                        child_title: "Accordion Item".to_string(),
                        child_copy: "Accordion content".to_string(),
                    }],
                    multiple: Some(false),
                })
            }
            ComponentKind::Alternating => {
                crate::model::SectionComponent::Alternating(crate::model::DdAlternating {
                    parent_type: crate::model::AlternatingType::Default,
                    parent_class: "-default".to_string(),
                    sal: crate::model::SalAnimation::Fade,
                    items: vec![crate::model::AlternatingItem {
                        child_image_url: "https://dummyimage.com/600x400/000/fff".to_string(),
                        child_image_alt: "Alternating image".to_string(),
                        child_title: "Alternating Item".to_string(),
                        child_copy: "Alternating content".to_string(),
                    }],
                })
            }
            ComponentKind::Card => crate::model::SectionComponent::Card(crate::model::DdCard {
                parent_type: crate::model::CardType::Default,
                sal: crate::model::SalAnimation::Fade,
                parent_width: "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24".to_string(),
                items: vec![crate::model::CardItem {
                    child_image_url: "https://dummyimage.com/720x720/000/fff".to_string(),
                    child_image_alt: "Image alt text".to_string(),
                    child_title: "Title".to_string(),
                    child_subtitle: "Subtitle".to_string(),
                    child_copy: "Copy".to_string(),
                    child_link_url: Some("/front".to_string()),
                    child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                    child_link_label: Some("Learn More".to_string()),
                }],
            }),
            ComponentKind::Filmstrip => {
                crate::model::SectionComponent::Filmstrip(crate::model::DdFilmstrip {
                    parent_type: crate::model::FilmstripType::Default,
                    sal: crate::model::SalAnimation::Fade,
                    items: vec![crate::model::FilmstripItem {
                        child_image_url: "https://dummyimage.com/256x256/000/fff".to_string(),
                        child_image_alt: "Image alt text".to_string(),
                        child_title: "Title".to_string(),
                    }],
                })
            }
            ComponentKind::Milestones => {
                crate::model::SectionComponent::Milestones(crate::model::DdMilestones {
                    sal: crate::model::SalAnimation::Fade,
                    parent_width: "dd-u-1-1 dd-u-md-12-24".to_string(),
                    items: vec![crate::model::MilestonesItem {
                        child_percentage: "70".to_string(),
                        child_title: "Title".to_string(),
                        child_subtitle: "Subtitle".to_string(),
                        child_copy: "Copy".to_string(),
                        child_link_url: None,
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: None,
                    }],
                })
            }
            ComponentKind::Modal => crate::model::SectionComponent::Modal(crate::model::DdModal {
                parent_title: "Title".to_string(),
                parent_copy: "Copy".to_string(),
            }),
            ComponentKind::Slider => {
                crate::model::SectionComponent::Slider(crate::model::DdSlider {
                    parent_title: String::new(),
                    items: vec![crate::model::SliderItem {
                        child_title: "Title".to_string(),
                        child_copy: "Copy".to_string(),
                        child_link_url: Some("/path".to_string()),
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_label: Some("Learn More".to_string()),
                        child_image_url: "https://dummyimage.com/720x720/000/fff".to_string(),
                        child_image_alt: "Image alt text".to_string(),
                    }],
                })
            }
            ComponentKind::Alert => crate::model::SectionComponent::Alert(crate::model::DdAlert {
                parent_type: crate::model::AlertType::Default,
                parent_class: crate::model::AlertClass::Default,
                sal: crate::model::SalAnimation::Fade,
                parent_title: Some("Alert Title".to_string()),
                parent_copy: "Alert content".to_string(),
            }),
            ComponentKind::Image => crate::model::SectionComponent::Image(crate::model::DdImage {
                sal: crate::model::SalAnimation::Fade,
                parent_image_url: "https://dummyimage.com/1200x600/000/fff".to_string(),
                parent_image_alt: "Image alt text".to_string(),
                parent_link_url: None,
                parent_link_target: None,
            }),
            ComponentKind::RichText => {
                crate::model::SectionComponent::RichText(crate::model::DdRichText {
                    parent_class: None,
                    sal: crate::model::SalAnimation::Fade,
                    parent_copy: "Copy".to_string(),
                })
            }
            ComponentKind::Navigation => {
                crate::model::SectionComponent::Navigation(crate::model::DdNavigation {
                    parent_type: crate::model::NavigationType::HeaderNav,
                    parent_class: crate::model::NavigationClass::MainMenu,
                    sal: crate::model::SalAnimation::Fade,
                    parent_width: "dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-18-24".to_string(),
                    items: vec![crate::model::NavigationItem {
                        child_kind: crate::model::NavigationKind::Link,
                        child_link_label: "Home".to_string(),
                        child_link_url: Some("/".to_string()),
                        child_link_target: Some(crate::model::CardLinkTarget::SelfTarget),
                        child_link_css: None,
                        items: Vec::new(),
                    }],
                })
            }
            ComponentKind::HeaderSearch => {
                crate::model::SectionComponent::HeaderSearch(crate::model::DdHeaderSearch {
                    parent_width: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24 dd-u-lg-4-24".to_string(),
                    sal: crate::model::SalAnimation::Fade,
                })
            }
            ComponentKind::HeaderMenu => {
                crate::model::SectionComponent::HeaderMenu(crate::model::DdHeaderMenu {
                    parent_width: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24".to_string(),
                    sal: crate::model::SalAnimation::Fade,
                })
            }
        }
    }
}
