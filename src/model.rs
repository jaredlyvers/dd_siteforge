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
    pub hero_class: Option<HeroImageClass>,
    pub hero_aos: Option<HeroAos>,
    pub title: String,
    pub subtitle: String,
    pub copy: Option<String>,
    pub cta_text: Option<String>,
    pub cta_link: Option<String>,
    pub cta_target: Option<CtaTarget>,
    pub cta_text_2: Option<String>,
    pub cta_link_2: Option<String>,
    pub cta_target_2: Option<CtaTarget>,
    pub image_alt: Option<String>,
    pub image_mobile: Option<String>,
    pub image_tablet: Option<String>,
    pub image_desktop: Option<String>,
    pub image_class: Option<HeroImageClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdSection {
    pub id: String,
    #[serde(default)]
    pub section_title: Option<String>,
    pub section_class: Option<SectionClass>,
    pub item_box_class: Option<SectionItemBoxClass>,
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
    Alternating(DdAlternating),
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
    #[serde(default = "default_alert_type", alias = "type")]
    pub alert_type: AlertType,
    #[serde(default = "default_alert_class")]
    pub alert_class: AlertClass,
    #[serde(default = "default_alert_data_aos", alias = "alert_aos")]
    pub alert_data_aos: HeroAos,
    #[serde(default, alias = "title")]
    pub alert_title: String,
    #[serde(default, alias = "message")]
    pub alert_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdAlternating {
    #[serde(default = "default_alternating_type")]
    pub alternating_type: AlternatingType,
    #[serde(default = "default_alternating_class")]
    pub alternating_class: String,
    #[serde(default = "default_alternating_data_aos")]
    pub alternating_data_aos: HeroAos,
    pub items: Vec<AlternatingItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternatingItem {
    pub image: String,
    pub image_alt: String,
    pub title: String,
    pub copy: String,
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
    #[serde(default = "default_accordion_type")]
    pub accordion_type: AccordionType,
    #[serde(default = "default_accordion_class")]
    pub accordion_class: AccordionClass,
    #[serde(default = "default_accordion_aos")]
    pub accordion_aos: HeroAos,
    #[serde(default = "default_accordion_group_name")]
    pub group_name: String,
    pub items: Vec<AccordionItem>,
    pub multiple: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccordionItem {
    pub title: String,
    pub content: String,
}

fn default_accordion_group_name() -> String {
    "group1".to_string()
}

fn default_accordion_type() -> AccordionType {
    AccordionType::Default
}

fn default_accordion_class() -> AccordionClass {
    AccordionClass::Primary
}

fn default_accordion_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_alert_type() -> AlertType {
    AlertType::Default
}

fn default_alert_class() -> AlertClass {
    AlertClass::Default
}

fn default_alert_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_alternating_type() -> AlternatingType {
    AlternatingType::Default
}

fn default_alternating_class() -> String {
    "-default".to_string()
}

fn default_alternating_data_aos() -> HeroAos {
    HeroAos::FadeIn
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeroImageClass {
    #[serde(rename = "-contained")]
    Contained,
    #[serde(rename = "-contained-md")]
    ContainedMd,
    #[serde(rename = "-contained-lg")]
    ContainedLg,
    #[serde(rename = "-contained-xl")]
    ContainedXl,
    #[serde(rename = "-contained-xxl")]
    ContainedXxl,
    #[serde(rename = "-full-full")]
    FullFull,
    #[serde(rename = "-full-contained")]
    FullContained,
    #[serde(rename = "-full-contained-md")]
    FullContainedMd,
    #[serde(rename = "-full-contained-lg")]
    FullContainedLg,
    #[serde(rename = "-full-contained-xl")]
    FullContainedXl,
    #[serde(rename = "-full-contained-xxl")]
    FullContainedXxl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeroAos {
    #[serde(rename = "fade-in")]
    FadeIn,
    #[serde(rename = "fade-up")]
    FadeUp,
    #[serde(rename = "fade-right")]
    FadeRight,
    #[serde(rename = "fade-down")]
    FadeDown,
    #[serde(rename = "fade-left")]
    FadeLeft,
    #[serde(rename = "zoom-in")]
    ZoomIn,
    #[serde(rename = "zoom-in-up")]
    ZoomInUp,
    #[serde(rename = "zoom-in-down")]
    ZoomInDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionClass {
    #[serde(rename = "-contained")]
    Contained,
    #[serde(rename = "-contained-md")]
    ContainedMd,
    #[serde(rename = "-contained-lg")]
    ContainedLg,
    #[serde(rename = "-contained-xl")]
    ContainedXl,
    #[serde(rename = "-contained-xxl")]
    ContainedXxl,
    #[serde(rename = "-full-full")]
    FullFull,
    #[serde(rename = "-full-contained")]
    FullContained,
    #[serde(rename = "-full-contained-md")]
    FullContainedMd,
    #[serde(rename = "-full-contained-lg")]
    FullContainedLg,
    #[serde(rename = "-full-contained-xl")]
    FullContainedXl,
    #[serde(rename = "-full-contained-xxl")]
    FullContainedXxl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SectionItemBoxClass {
    #[serde(rename = "l-box")]
    LBox,
    #[serde(rename = "ll-box")]
    LlBox,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-info -minor")]
    InfoMinor,
    #[serde(rename = "-warning -moderate -serious")]
    WarningModerateSerious,
    #[serde(rename = "-error -critical")]
    ErrorCritical,
    #[serde(rename = "-success")]
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertClass {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-compact")]
    Compact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlternatingType {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-reverse")]
    Reverse,
    #[serde(rename = "-no-alternate")]
    NoAlternate,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccordionType {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-faq")]
    Faq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccordionClass {
    #[serde(rename = "-borderless")]
    Borderless,
    #[serde(rename = "-compact")]
    Compact,
    #[serde(rename = "-primary")]
    Primary,
    #[serde(rename = "-secondary")]
    Secondary,
    #[serde(rename = "-tertiary")]
    Tertiary,
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
                        hero_class: Some(HeroImageClass::FullFull),
                        hero_aos: Some(HeroAos::FadeIn),
                        title: "Build with dd-framework".to_string(),
                        subtitle: "Framework-native static page builder".to_string(),
                        copy: Some("Compose pages with typed component schemas.".to_string()),
                        cta_text: Some("Get Started".to_string()),
                        cta_link: Some("/start".to_string()),
                        cta_target: Some(CtaTarget::SelfTarget),
                        cta_text_2: Some("Learn More".to_string()),
                        cta_link_2: Some("/learn-more".to_string()),
                        cta_target_2: Some(CtaTarget::SelfTarget),
                        image_alt: Some("Decorative hero".to_string()),
                        image_mobile: None,
                        image_tablet: None,
                        image_desktop: None,
                        image_class: Some(HeroImageClass::FullFull),
                    }),
                    PageNode::Section(DdSection {
                        id: "section-1".to_string(),
                        section_title: Some("Ready to publish?".to_string()),
                        section_class: Some(SectionClass::FullContained),
                        item_box_class: Some(SectionItemBoxClass::LBox),
                        columns: vec![SectionColumn {
                            id: "column-1".to_string(),
                            width_class: "dd-u-1-1".to_string(),
                            components: Vec::new(),
                        }],
                        components: Vec::new(),
                    }),
                ],
            }],
        }
    }
}
