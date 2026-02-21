use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub theme: ThemeSettings,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSettings {
    pub primary_color: String,
    pub secondary_color: String,
    pub tertiary_color: String,
    pub support_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub meta_description: Option<String>,
    pub nodes: Vec<PageNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_type", rename_all = "snake_case")]
pub enum PageNode {
    Hero(DdHero),
    Section(DdSection),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdHero {
    pub image: String,
    pub title: String,
    pub subtitle: String,
    pub copy: Option<String>,
    pub cta_text: Option<String>,
    pub cta_link: Option<String>,
    pub cta_target: Option<CtaTarget>,
    pub image_alt: Option<String>,
    pub image_mobile: Option<String>,
    pub image_tablet: Option<String>,
    pub image_desktop: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdSection {
    pub id: String,
    pub background: SectionBackground,
    pub spacing: SectionSpacing,
    pub width: SectionWidth,
    pub align: SectionAlign,
    #[serde(default)]
    pub columns: Vec<SectionColumn>,
    #[serde(default)]
    pub components: Vec<SectionComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionColumn {
    pub id: String,
    pub width_class: String,
    pub components: Vec<SectionComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "component_type", rename_all = "snake_case")]
pub enum SectionComponent {
    Card(DdCard),
    Alert(DdAlert),
    Banner(DdBanner),
    Tabs(DdTabs),
    Accordion(DdAccordion),
    Cta(DdCta),
    Modal(DdModal),
    Slider(DdSlider),
    Spacer(DdSpacer),
    Timeline(DdTimeline),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdCard {
    pub title: String,
    pub image: String,
    pub subtitle: Option<String>,
    pub copy: Option<String>,
    pub cta_text: Option<String>,
    pub cta_link: Option<String>,
    pub image_alt: Option<String>,
    pub columns: Option<CardColumns>,
    pub animate: Option<CardAnimate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdAlert {
    #[serde(rename = "type")]
    pub alert_type: AlertType,
    pub message: String,
    pub title: Option<String>,
    pub dismissible: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdBanner {
    pub message: String,
    pub background: String,
    pub link_text: Option<String>,
    pub link_url: Option<String>,
    pub dismissible: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdTabs {
    pub tabs: Vec<TabItem>,
    pub default_tab: Option<usize>,
    pub orientation: Option<TabsOrientation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabItem {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdAccordion {
    pub items: Vec<AccordionItem>,
    pub multiple: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccordionItem {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdCta {
    pub title: String,
    pub copy: String,
    pub cta_text: String,
    pub cta_link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdModal {
    pub trigger_text: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdSlider {
    pub slides: Vec<SlideItem>,
    pub autoplay: Option<bool>,
    pub speed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideItem {
    pub image: String,
    pub title: String,
    pub copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdSpacer {
    pub height: SpacerHeight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdTimeline {
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub date: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CtaTarget {
    #[serde(rename = "_self")]
    SelfTarget,
    #[serde(rename = "_blank")]
    Blank,
    #[serde(rename = "_parent")]
    Parent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionBackground {
    Primary,
    Secondary,
    Tertiary,
    Gray,
    White,
    Black,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionSpacing {
    Tight,
    Normal,
    Loose,
    ExtraLoose,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionWidth {
    Narrow,
    Normal,
    Wide,
    Full,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CardColumns {
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "3")]
    Three,
    #[serde(rename = "4")]
    Four,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CardAnimate {
    FadeUp,
    FadeIn,
    SlideUp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    Success,
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabsOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpacerHeight {
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

impl Site {
    pub fn starter() -> Self {
        Self {
            schema_version: 1,
            id: "site-1".to_string(),
            name: "My Site".to_string(),
            theme: ThemeSettings {
                primary_color: "#88d9f7".to_string(),
                secondary_color: "#ffca76".to_string(),
                tertiary_color: "#f98971".to_string(),
                support_color: "#46be8c".to_string(),
            },
            pages: vec![Page {
                id: "page-home".to_string(),
                slug: "index".to_string(),
                title: "Home".to_string(),
                meta_description: Some("Starter page".to_string()),
                nodes: vec![
                    PageNode::Hero(DdHero {
                        image: "/assets/images/hero.jpg".to_string(),
                        title: "Build with dd-framework".to_string(),
                        subtitle: "Framework-native static page builder".to_string(),
                        copy: Some("Compose pages with typed component schemas.".to_string()),
                        cta_text: Some("Get Started".to_string()),
                        cta_link: Some("/start".to_string()),
                        cta_target: Some(CtaTarget::SelfTarget),
                        image_alt: Some("Decorative hero".to_string()),
                        image_mobile: None,
                        image_tablet: None,
                        image_desktop: None,
                    }),
                    PageNode::Section(DdSection {
                        id: "section-1".to_string(),
                        background: SectionBackground::White,
                        spacing: SectionSpacing::Normal,
                        width: SectionWidth::Normal,
                        align: SectionAlign::Left,
                        columns: vec![SectionColumn {
                            id: "column-1".to_string(),
                            width_class: "dd-u-1-1".to_string(),
                            components: vec![SectionComponent::Cta(DdCta {
                                title: "Ready to publish?".to_string(),
                                copy: "Generate framework-compliant HTML.".to_string(),
                                cta_text: "Export".to_string(),
                                cta_link: "/export".to_string(),
                            })],
                        }],
                        components: Vec::new(),
                    }),
                ],
            }],
        }
    }
}
