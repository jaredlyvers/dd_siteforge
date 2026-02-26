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
    pub custom_css: Option<String>,
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
    Alternating(DdAlternating),
    Card(DdCard),
    Cta(DdCta),
    Filmstrip(DdFilmstrip),
    Milestones(DdMilestones),
    Banner(DdBanner),
    Accordion(DdAccordion),
    Blockquote(DdBlockquote),
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
pub struct DdCard {
    #[serde(default = "default_card_type")]
    pub card_type: CardType,
    #[serde(default = "default_card_data_aos")]
    pub card_data_aos: HeroAos,
    #[serde(default = "default_card_width")]
    pub card_width: String,
    pub items: Vec<CardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardItem {
    pub card_image_url: String,
    pub card_image_alt: String,
    pub card_title: String,
    pub card_subtitle: String,
    pub card_copy: String,
    pub card_link_url: Option<String>,
    pub card_link_target: Option<CardLinkTarget>,
    pub card_link_label: Option<String>,
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
    #[serde(default = "default_banner_class")]
    pub banner_class: BannerClass,
    #[serde(default = "default_banner_data_aos")]
    pub banner_data_aos: HeroAos,
    pub banner_image_url: String,
    pub banner_image_alt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdCta {
    #[serde(default = "default_cta_class")]
    pub cta_class: CtaClass,
    pub cta_image_url: String,
    pub cta_image_alt: String,
    #[serde(default = "default_cta_data_aos")]
    pub cta_data_aos: HeroAos,
    pub cta_title: String,
    pub cta_subtitle: String,
    pub cta_copy: String,
    pub cta_link_url: Option<String>,
    pub cta_link_target: Option<CardLinkTarget>,
    pub cta_link_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdFilmstrip {
    #[serde(default = "default_filmstrip_type")]
    pub filmstrip_type: FilmstripType,
    #[serde(default = "default_filmstrip_data_aos")]
    pub filmstrip_data_aos: HeroAos,
    pub items: Vec<FilmstripItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmstripItem {
    pub image_url: String,
    pub image_alt: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdMilestones {
    #[serde(default = "default_milestones_data_aos")]
    pub parent_data_aos: HeroAos,
    #[serde(default = "default_milestones_width")]
    pub parent_width: String,
    pub items: Vec<MilestonesItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MilestonesItem {
    pub child_percentage: String,
    pub child_title: String,
    pub child_subtitle: String,
    pub child_copy: String,
    pub child_link_url: Option<String>,
    pub child_link_target: Option<CardLinkTarget>,
    pub child_link_label: Option<String>,
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
pub struct DdBlockquote {
    #[serde(default = "default_blockquote_data_aos")]
    pub blockquote_data_aos: HeroAos,
    pub blockquote_image_url: String,
    pub blockquote_image_alt: String,
    pub blockquote_persons_name: String,
    pub blockquote_persons_title: String,
    pub blockquote_copy: String,
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

fn default_alternating_type() -> AlternatingType {
    AlternatingType::Default
}

fn default_alternating_class() -> String {
    "-default".to_string()
}

fn default_alternating_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_card_type() -> CardType {
    CardType::Default
}

fn default_card_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_card_width() -> String {
    "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24".to_string()
}

fn default_banner_class() -> BannerClass {
    BannerClass::BgCenterCenter
}

fn default_banner_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_cta_class() -> CtaClass {
    CtaClass::TopLeft
}

fn default_cta_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_filmstrip_type() -> FilmstripType {
    FilmstripType::Default
}

fn default_filmstrip_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_milestones_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_milestones_width() -> String {
    "dd-u-1-1 dd-u-md-12-24".to_string()
}

fn default_blockquote_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlternatingType {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-reverse")]
    Reverse,
    #[serde(rename = "-no-alternate")]
    NoAlternate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-horizontal")]
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardLinkTarget {
    #[serde(rename = "_self")]
    SelfTarget,
    #[serde(rename = "_blank")]
    Blank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BannerClass {
    #[serde(rename = "-bg-top-left")]
    BgTopLeft,
    #[serde(rename = "-bg-top-center")]
    BgTopCenter,
    #[serde(rename = "-bg-top-right")]
    BgTopRight,
    #[serde(rename = "-bg-center-left")]
    BgCenterLeft,
    #[serde(rename = "-bg-center-center")]
    BgCenterCenter,
    #[serde(rename = "-bg-center-right")]
    BgCenterRight,
    #[serde(rename = "-bg-bottom-left")]
    BgBottomLeft,
    #[serde(rename = "-bg-bottom-center")]
    BgBottomCenter,
    #[serde(rename = "-bg-bottom-right")]
    BgBottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtaClass {
    #[serde(rename = "-top-left")]
    TopLeft,
    #[serde(rename = "-top-center")]
    TopCenter,
    #[serde(rename = "-top-right")]
    TopRight,
    #[serde(rename = "-center-left")]
    CenterLeft,
    #[serde(rename = "-center-center")]
    CenterCenter,
    #[serde(rename = "-center-right")]
    CenterRight,
    #[serde(rename = "-bottom-left")]
    BottomLeft,
    #[serde(rename = "-bottom-center")]
    BottomCenter,
    #[serde(rename = "-bottom-right")]
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilmstripType {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-reverse")]
    Reverse,
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
                        custom_css: None,
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
