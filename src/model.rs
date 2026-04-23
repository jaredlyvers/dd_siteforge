use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub theme: ThemeSettings,
    pub header: DdHeader,
    pub footer: DdFooter,
    pub pages: Vec<Page>,
    /// Persisted export output directory, relative to the site JSON file.
    /// `None` triggers a first-time prompt; user-confirmed value is written back.
    #[serde(default)]
    pub export_dir: Option<String>,
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
    #[serde(default)]
    pub slug_locked: bool,
    pub head: DdHead,
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
    pub parent_image_url: String,
    pub parent_image_alt: Option<String>,
    pub parent_class: Option<HeroImageClass>,
    pub parent_data_aos: Option<HeroAos>,
    pub parent_custom_css: Option<String>,
    pub parent_title: String,
    pub parent_subtitle: String,
    pub parent_copy: Option<String>,
    pub link_1_label: Option<String>,
    pub link_1_url: Option<String>,
    pub link_1_target: Option<CtaTarget>,
    pub link_2_label: Option<String>,
    pub link_2_url: Option<String>,
    pub link_2_target: Option<CtaTarget>,
    pub parent_image_mobile: Option<String>,
    pub parent_image_tablet: Option<String>,
    pub parent_image_desktop: Option<String>,
    pub parent_image_class: Option<HeroImageClass>,
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
    Slider(DdSlider),
    Modal(DdModal),
    Banner(DdBanner),
    Accordion(DdAccordion),
    Blockquote(DdBlockquote),
    Alert(DdAlert),
    Image(DdImage),
    RichText(DdRichText),
    Navigation(DdNavigation),
    HeaderSearch(DdHeaderSearch),
    HeaderMenu(DdHeaderMenu),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdAlternating {
    #[serde(default = "default_alternating_parent_type")]
    pub parent_type: AlternatingType,
    #[serde(default = "default_alternating_parent_class")]
    pub parent_class: String,
    #[serde(default = "default_alternating_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub items: Vec<AlternatingItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternatingItem {
    pub child_image_url: String,
    pub child_image_alt: String,
    pub child_title: String,
    pub child_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdCard {
    #[serde(default = "default_card_parent_type")]
    pub parent_type: CardType,
    #[serde(default = "default_card_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    #[serde(default = "default_card_parent_width")]
    pub parent_width: String,
    pub items: Vec<CardItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardItem {
    pub child_image_url: String,
    pub child_image_alt: String,
    pub child_title: String,
    pub child_subtitle: String,
    pub child_copy: String,
    pub child_link_url: Option<String>,
    pub child_link_target: Option<CardLinkTarget>,
    pub child_link_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdBanner {
    #[serde(default = "default_banner_parent_class")]
    pub parent_class: BannerClass,
    #[serde(default = "default_banner_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub parent_image_url: String,
    pub parent_image_alt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdCta {
    #[serde(default = "default_cta_parent_class")]
    pub parent_class: CtaClass,
    pub parent_image_url: String,
    pub parent_image_alt: String,
    #[serde(default = "default_cta_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub parent_title: String,
    pub parent_subtitle: String,
    pub parent_copy: String,
    pub parent_link_url: Option<String>,
    pub parent_link_target: Option<CardLinkTarget>,
    pub parent_link_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdFilmstrip {
    #[serde(default = "default_filmstrip_parent_type")]
    pub parent_type: FilmstripType,
    #[serde(default = "default_filmstrip_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub items: Vec<FilmstripItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilmstripItem {
    pub child_image_url: String,
    pub child_image_alt: String,
    pub child_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdMilestones {
    #[serde(default = "default_milestones_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    #[serde(default = "default_milestones_parent_width")]
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
pub struct DdModal {
    pub parent_title: String,
    pub parent_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdSlider {
    #[serde(default)]
    pub parent_title: String,
    pub items: Vec<SliderItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliderItem {
    pub child_title: String,
    pub child_copy: String,
    pub child_link_url: Option<String>,
    pub child_link_target: Option<CardLinkTarget>,
    pub child_link_label: Option<String>,
    pub child_image_url: String,
    pub child_image_alt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdAccordion {
    #[serde(default = "default_accordion_parent_type")]
    pub parent_type: AccordionType,
    #[serde(default = "default_accordion_parent_class")]
    pub parent_class: AccordionClass,
    #[serde(default = "default_accordion_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    #[serde(default = "default_accordion_parent_group_name")]
    pub parent_group_name: String,
    pub items: Vec<AccordionItem>,
    pub multiple: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccordionItem {
    pub child_title: String,
    pub child_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdAlert {
    #[serde(default = "default_alert_parent_type")]
    pub parent_type: AlertType,
    #[serde(default = "default_alert_parent_class")]
    pub parent_class: AlertClass,
    #[serde(default = "default_alert_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub parent_title: Option<String>,
    pub parent_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdBlockquote {
    #[serde(default = "default_blockquote_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub parent_image_url: String,
    pub parent_image_alt: String,
    pub parent_name: String,
    pub parent_role: String,
    pub parent_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdImage {
    #[serde(default = "default_image_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub parent_image_url: String,
    pub parent_image_alt: String,
    pub parent_link_url: Option<String>,
    pub parent_link_target: Option<CardLinkTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdRichText {
    #[serde(default)]
    pub parent_class: Option<String>,
    #[serde(default = "default_rich_text_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    pub parent_copy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdNavigation {
    #[serde(default = "default_navigation_parent_type")]
    pub parent_type: NavigationType,
    #[serde(default = "default_navigation_parent_class")]
    pub parent_class: NavigationClass,
    #[serde(default = "default_navigation_parent_data_aos")]
    pub parent_data_aos: HeroAos,
    #[serde(default = "default_navigation_parent_width")]
    pub parent_width: String,
    pub items: Vec<NavigationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationItem {
    #[serde(default = "default_navigation_child_kind")]
    pub child_kind: NavigationKind,
    pub child_link_label: String,
    pub child_link_url: Option<String>,
    pub child_link_target: Option<CardLinkTarget>,
    pub child_link_css: Option<String>,
    #[serde(default)]
    pub items: Vec<NavigationItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdHeaderSearch {
    #[serde(default = "default_header_search_parent_width")]
    pub parent_width: String,
    #[serde(default = "default_header_search_parent_data_aos")]
    pub parent_data_aos: HeroAos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdHeaderMenu {
    #[serde(default = "default_header_menu_parent_width")]
    pub parent_width: String,
    #[serde(default = "default_header_menu_parent_data_aos")]
    pub parent_data_aos: HeroAos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdHeader {
    pub id: String,
    pub custom_css: Option<String>,
    #[serde(default)]
    pub alert: Option<DdAlert>,
    #[serde(default)]
    pub sections: Vec<DdSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdFooter {
    pub id: String,
    pub custom_css: Option<String>,
    #[serde(default)]
    pub sections: Vec<DdSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdHead {
    pub title: String,
    pub meta_description: Option<String>,
    pub canonical_url: Option<String>,
    #[serde(default = "default_head_robots")]
    pub robots: RobotsDirective,
    #[serde(default = "default_head_schema_type")]
    pub schema_type: SchemaType,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
}

fn default_alert_parent_type() -> AlertType {
    AlertType::Default
}

fn default_alert_parent_class() -> AlertClass {
    AlertClass::Default
}

fn default_alert_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_accordion_parent_group_name() -> String {
    "group1".to_string()
}

fn default_accordion_parent_type() -> AccordionType {
    AccordionType::Default
}

fn default_accordion_parent_class() -> AccordionClass {
    AccordionClass::Primary
}

fn default_accordion_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_alternating_parent_type() -> AlternatingType {
    AlternatingType::Default
}

fn default_alternating_parent_class() -> String {
    "-default".to_string()
}

fn default_alternating_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_card_parent_type() -> CardType {
    CardType::Default
}

fn default_card_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_card_parent_width() -> String {
    "dd-u-1-1 dd-u-md-12-24 dd-u-lg-8-24".to_string()
}

fn default_banner_parent_class() -> BannerClass {
    BannerClass::BgCenterCenter
}

fn default_banner_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_cta_parent_class() -> CtaClass {
    CtaClass::TopLeft
}

fn default_cta_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_filmstrip_parent_type() -> FilmstripType {
    FilmstripType::Default
}

fn default_filmstrip_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_milestones_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_milestones_parent_width() -> String {
    "dd-u-1-1 dd-u-md-12-24".to_string()
}

fn default_blockquote_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_image_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_rich_text_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_navigation_parent_type() -> NavigationType {
    NavigationType::HeaderNav
}

fn default_navigation_parent_class() -> NavigationClass {
    NavigationClass::MainMenu
}

fn default_navigation_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_navigation_parent_width() -> String {
    "dd-u-1-1 dd-u-sm-1-1 dd-u-md-1-1 dd-u-lg-18-24".to_string()
}

fn default_navigation_child_kind() -> NavigationKind {
    NavigationKind::Link
}

fn default_header_search_parent_width() -> String {
    "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24 dd-u-lg-4-24".to_string()
}

fn default_header_search_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_header_menu_parent_width() -> String {
    "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24".to_string()
}

fn default_header_menu_parent_data_aos() -> HeroAos {
    HeroAos::FadeIn
}

fn default_head_robots() -> RobotsDirective {
    RobotsDirective::IndexFollow
}

fn default_head_schema_type() -> SchemaType {
    SchemaType::WebPage
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    #[serde(rename = "-default")]
    Default,
    #[serde(rename = "-info")]
    Info,
    #[serde(rename = "-warning")]
    Warning,
    #[serde(rename = "-error")]
    Error,
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
pub enum NavigationKind {
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "button")]
    Button,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationType {
    #[serde(rename = "dd-header__navigation")]
    HeaderNav,
    #[serde(rename = "dd-footer__navigation")]
    FooterNav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NavigationClass {
    #[serde(rename = "-main-menu")]
    MainMenu,
    #[serde(rename = "-menu-secondary")]
    MenuSecondary,
    #[serde(rename = "-menu-tertiary")]
    MenuTertiary,
    #[serde(rename = "-footer-menu")]
    FooterMenu,
    #[serde(rename = "-footer-menu-secondary")]
    FooterMenuSecondary,
    #[serde(rename = "-footer-menu-tertiary")]
    FooterMenuTertiary,
    #[serde(rename = "-social-menu")]
    SocialMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotsDirective {
    #[serde(rename = "index, follow")]
    IndexFollow,
    #[serde(rename = "noindex, follow")]
    NoindexFollow,
    #[serde(rename = "index, nofollow")]
    IndexNofollow,
    #[serde(rename = "noindex, nofollow")]
    NoindexNofollow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemaType {
    WebPage,
    Article,
    AboutPage,
    ContactPage,
    CollectionPage,
    Organization,
    LocalBusiness,
    Product,
    Service,
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
            header: DdHeader {
                id: "header".to_string(),
                custom_css: None,
                alert: None,
                sections: vec![DdSection {
                    id: "header-section-1".to_string(),
                    section_title: None,
                    section_class: Some(SectionClass::FullContained),
                    item_box_class: Some(SectionItemBoxClass::LBox),
                    columns: vec![
                        SectionColumn {
                            id: "column-1".to_string(),
                            width_class: "dd-u-18-24 dd-u-md-18-24".to_string(),
                            components: Vec::new(),
                        },
                        SectionColumn {
                            id: "column-2".to_string(),
                            width_class: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24 dd-u-lg-4-24"
                                .to_string(),
                            components: vec![SectionComponent::HeaderSearch(DdHeaderSearch {
                                parent_width: default_header_search_parent_width(),
                                parent_data_aos: HeroAos::FadeIn,
                            })],
                        },
                        SectionColumn {
                            id: "column-3".to_string(),
                            width_class: "dd-u-3-24 dd-u-sm-3-24 dd-u-md-3-24".to_string(),
                            components: vec![SectionComponent::HeaderMenu(DdHeaderMenu {
                                parent_width: default_header_menu_parent_width(),
                                parent_data_aos: HeroAos::FadeIn,
                            })],
                        },
                    ],
                }],
            },
            footer: DdFooter {
                id: "footer".to_string(),
                custom_css: None,
                sections: vec![DdSection {
                    id: "footer-section-1".to_string(),
                    section_title: None,
                    section_class: Some(SectionClass::FullContained),
                    item_box_class: Some(SectionItemBoxClass::LBox),
                    columns: vec![SectionColumn {
                        id: "column-1".to_string(),
                        width_class: "dd-u-1-1".to_string(),
                        components: Vec::new(),
                    }],
                }],
            },
            export_dir: None,
            pages: vec![Page {
                id: "page-home".to_string(),
                slug: "index".to_string(),
                slug_locked: false,
                head: DdHead {
                    title: "Home".to_string(),
                    meta_description: Some("Starter page".to_string()),
                    canonical_url: None,
                    robots: RobotsDirective::IndexFollow,
                    schema_type: SchemaType::WebPage,
                    og_title: None,
                    og_description: None,
                    og_image: None,
                },
                nodes: vec![
                    PageNode::Hero(DdHero {
                        parent_image_url: "/assets/images/hero.jpg".to_string(),
                        parent_image_alt: Some("Decorative hero".to_string()),
                        parent_class: Some(HeroImageClass::FullFull),
                        parent_data_aos: Some(HeroAos::FadeIn),
                        parent_custom_css: None,
                        parent_title: "Build with dd-framework".to_string(),
                        parent_subtitle: "Framework-native static page builder".to_string(),
                        parent_copy: Some(
                            "Compose pages with typed component schemas.".to_string(),
                        ),
                        link_1_label: Some("Get Started".to_string()),
                        link_1_url: Some("/start".to_string()),
                        link_1_target: Some(CtaTarget::SelfTarget),
                        link_2_label: Some("Learn More".to_string()),
                        link_2_url: Some("/learn-more".to_string()),
                        link_2_target: Some(CtaTarget::SelfTarget),
                        parent_image_mobile: None,
                        parent_image_tablet: None,
                        parent_image_desktop: None,
                        parent_image_class: Some(HeroImageClass::FullFull),
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
                    }),
                ],
            }],
        }
    }
}

/// Starter content to seed a new page with.
#[derive(Debug, Clone, Copy)]
pub enum PageTemplate {
    /// No nodes, just the `[HEAD]` metadata.
    Blank,
    /// One empty `dd-hero`.
    HeroOnly,
    /// `dd-hero` + `dd-section` with one empty column.
    HeroPlusSection,
}

impl Page {
    pub fn from_template(title: &str, template: PageTemplate) -> Self {
        let slug = slug_from_title(title);
        let id = format!("page-{}", slug);
        let nodes = match template {
            PageTemplate::Blank => Vec::new(),
            PageTemplate::HeroOnly => vec![PageNode::Hero(Self::empty_hero())],
            PageTemplate::HeroPlusSection => vec![
                PageNode::Hero(Self::empty_hero()),
                PageNode::Section(Self::empty_section()),
            ],
        };
        Page {
            id,
            slug,
            slug_locked: false,
            head: DdHead {
                title: title.to_string(),
                meta_description: None,
                canonical_url: None,
                robots: RobotsDirective::IndexFollow,
                schema_type: SchemaType::WebPage,
                og_title: None,
                og_description: None,
                og_image: None,
            },
            nodes,
        }
    }

    pub fn duplicate_from(src: &Page) -> Self {
        let title = format!("{} (Copy)", src.head.title);
        let slug = slug_from_title(&title);
        let mut head = src.head.clone();
        head.title = title;
        Page {
            id: format!("page-{}", slug),
            slug,
            slug_locked: false,
            head,
            nodes: src.nodes.clone(),
        }
    }

    fn empty_hero() -> DdHero {
        DdHero {
            parent_image_url: String::new(),
            parent_image_alt: None,
            parent_class: Some(HeroImageClass::FullFull),
            parent_data_aos: Some(HeroAos::FadeIn),
            parent_custom_css: None,
            parent_title: String::new(),
            parent_subtitle: String::new(),
            parent_copy: None,
            link_1_label: None,
            link_1_url: None,
            link_1_target: None,
            link_2_label: None,
            link_2_url: None,
            link_2_target: None,
            parent_image_mobile: None,
            parent_image_tablet: None,
            parent_image_desktop: None,
            parent_image_class: Some(HeroImageClass::FullFull),
        }
    }

    fn empty_section() -> DdSection {
        DdSection {
            id: "section-1".to_string(),
            section_title: None,
            section_class: Some(SectionClass::FullContained),
            item_box_class: Some(SectionItemBoxClass::LBox),
            columns: vec![SectionColumn {
                id: "column-1".to_string(),
                width_class: "dd-u-1-1".to_string(),
                components: Vec::new(),
            }],
        }
    }
}

/// Convert a human title to a filesystem/URL-safe kebab-case slug.
/// ASCII-only, lowercase, alphanumerics and hyphens preserved,
/// whitespace collapsed to single `-`, everything else stripped.
/// Falls back to `"untitled"` for empty/whitespace-only inputs.
pub fn slug_from_title(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_hyphen = false;
    for ch in title.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_hyphen = false;
        } else if c.is_whitespace() || c == '-' || c == '_' {
            if !prev_hyphen && !out.is_empty() {
                out.push('-');
                prev_hyphen = true;
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "untitled".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_from_template_blank_has_no_nodes() {
        let p = Page::from_template("Contact Us", PageTemplate::Blank);
        assert_eq!(p.head.title, "Contact Us");
        assert_eq!(p.slug, "contact-us");
        assert!(!p.slug_locked);
        assert!(p.nodes.is_empty());
    }

    #[test]
    fn page_from_template_hero_only_has_one_hero_node() {
        let p = Page::from_template("Gallery", PageTemplate::HeroOnly);
        assert_eq!(p.nodes.len(), 1);
        assert!(matches!(p.nodes[0], PageNode::Hero(_)));
    }

    #[test]
    fn page_from_template_hero_plus_section_has_hero_then_section() {
        let p = Page::from_template("Services", PageTemplate::HeroPlusSection);
        assert_eq!(p.nodes.len(), 2);
        assert!(matches!(p.nodes[0], PageNode::Hero(_)));
        assert!(matches!(p.nodes[1], PageNode::Section(_)));
    }

    #[test]
    fn page_from_template_duplicate_deep_clones_and_appends_copy_suffix() {
        let mut starter = Site::starter();
        starter.pages[0].head.title = "Home".to_string();
        let dup = Page::duplicate_from(&starter.pages[0]);
        assert_eq!(dup.head.title, "Home (Copy)");
        assert_eq!(dup.slug, "home-copy");
        assert!(!dup.slug_locked);
        assert_eq!(dup.nodes.len(), starter.pages[0].nodes.len());
        assert_ne!(dup.id, starter.pages[0].id);
    }

    #[test]
    fn slug_from_title_basic_lowercase_and_hyphenate() {
        assert_eq!(slug_from_title("Contact Us"), "contact-us");
    }

    #[test]
    fn slug_from_title_strips_punctuation() {
        assert_eq!(slug_from_title("Hello, World!"), "hello-world");
    }

    #[test]
    fn slug_from_title_collapses_whitespace() {
        assert_eq!(slug_from_title("  a   b  "), "a-b");
    }

    #[test]
    fn slug_from_title_empty_fallback() {
        assert_eq!(slug_from_title(""), "untitled");
        assert_eq!(slug_from_title("   "), "untitled");
        assert_eq!(slug_from_title("!!!"), "untitled");
    }

    #[test]
    fn slug_from_title_preserves_existing_hyphens_without_duplicating() {
        assert_eq!(slug_from_title("about-us"), "about-us");
        assert_eq!(slug_from_title("about  -  us"), "about-us");
    }
}
