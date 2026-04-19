//! Region-aware mutation cursor.
//!
//! A `Cursor` identifies any editable node anywhere in the site — header,
//! footer, or page. `resolve_mut()` converts a cursor into a mutable
//! reference to the underlying model node. `apply_edit_form_to_component()`
//! is the single entry point the editor calls on Ctrl+S; it writes every
//! visible field of an `EditFormState` back into the target, correctly
//! routing to the header/footer/page region.
//!
//! This module is the structural fix for the "header/footer edits
//! silently target the current page" bug: every write path funnels through
//! `resolve_mut`, which knows every region.

use anyhow::{anyhow, Context, Result};

use crate::model::{
    AlertClass, AlertType, BannerClass, CardLinkTarget, CtaClass, DdAlert, DdBanner, DdBlockquote,
    DdCta, DdFooter, DdHead, DdHeader, DdHeaderMenu, DdHeaderSearch, DdHero, DdImage, DdModal,
    DdRichText, DdSection, HeroAos, PageNode, SectionColumn, SectionComponent, Site,
};
use crate::tui::editform::{self, EditFormState, FieldKind};

/// Address of any editable node in the site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    // --- Header region ---
    HeaderRoot,
    HeaderSection {
        sec: usize,
    },
    HeaderComponent {
        sec: usize,
        col: usize,
        comp: usize,
        items: Vec<usize>,
    },
    // --- Footer region ---
    FooterRoot,
    FooterSection {
        sec: usize,
    },
    FooterComponent {
        sec: usize,
        col: usize,
        comp: usize,
        items: Vec<usize>,
    },
    // --- Page region ---
    PageHead {
        page: usize,
    },
    PageHero {
        page: usize,
        node: usize,
    },
    PageSection {
        page: usize,
        node: usize,
    },
    PageComponent {
        page: usize,
        node: usize,
        col: usize,
        comp: usize,
        items: Vec<usize>,
    },
}

/// Typed mutable reference to whichever node a `Cursor` resolved to.
#[allow(dead_code)] // most variants unused until Tier A/B/C/D migrations
pub enum CursorRef<'a> {
    Hero(&'a mut DdHero),
    Section(&'a mut DdSection),
    Component(&'a mut SectionComponent),
    Head(&'a mut DdHead),
    HeaderRoot(&'a mut DdHeader),
    FooterRoot(&'a mut DdFooter),
}

/// Resolve a cursor to a mutable typed reference inside the site.
pub fn resolve_mut<'a>(site: &'a mut Site, cursor: &Cursor) -> Result<CursorRef<'a>> {
    match cursor {
        Cursor::HeaderRoot => Ok(CursorRef::HeaderRoot(&mut site.header)),
        Cursor::HeaderSection { sec } => {
            let s = site
                .header
                .sections
                .get_mut(*sec)
                .context("header section index out of bounds")?;
            Ok(CursorRef::Section(s))
        }
        Cursor::HeaderComponent { sec, col, comp, .. } => {
            let column = resolve_column_mut(&mut site.header.sections, *sec, *col)?;
            let c = column
                .components
                .get_mut(*comp)
                .context("header component index out of bounds")?;
            Ok(CursorRef::Component(c))
        }
        Cursor::FooterRoot => Ok(CursorRef::FooterRoot(&mut site.footer)),
        Cursor::FooterSection { sec } => {
            let s = site
                .footer
                .sections
                .get_mut(*sec)
                .context("footer section index out of bounds")?;
            Ok(CursorRef::Section(s))
        }
        Cursor::FooterComponent { sec, col, comp, .. } => {
            let column = resolve_column_mut(&mut site.footer.sections, *sec, *col)?;
            let c = column
                .components
                .get_mut(*comp)
                .context("footer component index out of bounds")?;
            Ok(CursorRef::Component(c))
        }
        Cursor::PageHead { page } => {
            let p = site
                .pages
                .get_mut(*page)
                .context("page index out of bounds")?;
            Ok(CursorRef::Head(&mut p.head))
        }
        Cursor::PageHero { page, node } => {
            let p = site
                .pages
                .get_mut(*page)
                .context("page index out of bounds")?;
            let n = p
                .nodes
                .get_mut(*node)
                .context("page node index out of bounds")?;
            match n {
                PageNode::Hero(h) => Ok(CursorRef::Hero(h)),
                _ => Err(anyhow!("cursor points at hero but node is not a Hero")),
            }
        }
        Cursor::PageSection { page, node } => {
            let p = site
                .pages
                .get_mut(*page)
                .context("page index out of bounds")?;
            let n = p
                .nodes
                .get_mut(*node)
                .context("page node index out of bounds")?;
            match n {
                PageNode::Section(s) => Ok(CursorRef::Section(s)),
                _ => Err(anyhow!("cursor points at section but node is not a Section")),
            }
        }
        Cursor::PageComponent {
            page,
            node,
            col,
            comp,
            ..
        } => {
            let p = site
                .pages
                .get_mut(*page)
                .context("page index out of bounds")?;
            let n = p
                .nodes
                .get_mut(*node)
                .context("page node index out of bounds")?;
            let section = match n {
                PageNode::Section(s) => s,
                _ => return Err(anyhow!("component cursor does not address a Section node")),
            };
            let column = section
                .columns
                .get_mut(*col)
                .context("column index out of bounds")?;
            let c = column
                .components
                .get_mut(*comp)
                .context("component index out of bounds")?;
            Ok(CursorRef::Component(c))
        }
    }
}

fn resolve_column_mut<'a>(
    sections: &'a mut [DdSection],
    sec_idx: usize,
    col_idx: usize,
) -> Result<&'a mut SectionColumn> {
    let s = sections
        .get_mut(sec_idx)
        .context("section index out of bounds")?;
    s.columns
        .get_mut(col_idx)
        .context("column index out of bounds")
}

/// Apply every visible field of `state` back into the model node at `cursor`.
/// The single entry point for Ctrl+S from the form editor.
pub fn apply_edit_form_to_component(
    site: &mut Site,
    cursor: &Cursor,
    state: &EditFormState,
) -> Result<()> {
    let target = resolve_mut(site, cursor)?;
    match target {
        CursorRef::Component(component) => match component {
            SectionComponent::Cta(cta) => apply_cta_values(cta, state),
            SectionComponent::Banner(b) => apply_banner_values(b, state),
            SectionComponent::Image(i) => apply_image_values(i, state),
            SectionComponent::HeaderSearch(h) => apply_header_search_values(h, state),
            SectionComponent::HeaderMenu(h) => apply_header_menu_values(h, state),
            SectionComponent::RichText(r) => apply_rich_text_values(r, state),
            SectionComponent::Alert(a) => apply_alert_values(a, state),
            SectionComponent::Modal(m) => apply_modal_values(m, state),
            SectionComponent::Blockquote(bq) => apply_blockquote_values(bq, state),
            other => Err(anyhow!(
                "apply_edit_form_to_component: component type not migrated to unified editor yet ({:?})",
                std::mem::discriminant(other)
            )),
        },
        other => Err(anyhow!(
            "apply_edit_form_to_component: unsupported cursor target (kind index={})",
            cursor_ref_kind(&other)
        )),
    }
}

fn cursor_ref_kind(r: &CursorRef) -> u8 {
    match r {
        CursorRef::Hero(_) => 0,
        CursorRef::Section(_) => 1,
        CursorRef::Component(_) => 2,
        CursorRef::Head(_) => 3,
        CursorRef::HeaderRoot(_) => 4,
        CursorRef::FooterRoot(_) => 5,
    }
}

fn apply_cta_values(cta: &mut DdCta, state: &EditFormState) -> Result<()> {
    cta.parent_class = parse_enum::<CtaClass>(state.get("parent_class"))
        .context("invalid parent_class")?;
    cta.parent_image_url = state.get("parent_image_url").trim().to_string();
    cta.parent_image_alt = state.get("parent_image_alt").trim().to_string();
    cta.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    cta.parent_title = state.get("parent_title").to_string();
    cta.parent_subtitle = state.get("parent_subtitle").to_string();
    cta.parent_copy = state.get("parent_copy").to_string();

    let link_url_raw = state.get("parent_link_url").trim().to_string();
    let link_target_raw = state.get("parent_link_target").to_string();
    let link_label_raw = state.get("parent_link_label").trim().to_string();
    if link_url_raw.is_empty() && link_label_raw.is_empty() {
        cta.parent_link_url = None;
        cta.parent_link_target = None;
        cta.parent_link_label = None;
    } else {
        cta.parent_link_url = Some(link_url_raw);
        cta.parent_link_target = Some(
            parse_enum::<CardLinkTarget>(&link_target_raw)
                .context("invalid parent_link_target")?,
        );
        cta.parent_link_label = Some(link_label_raw);
    }
    Ok(())
}

/// Entry point used by the tui to turn a live component into an `EditFormState`.
/// Returns None when the component type hasn't been migrated to the unified
/// editor yet.
pub fn component_to_form_state(component: &SectionComponent) -> Option<EditFormState> {
    match component {
        SectionComponent::Cta(c) => Some(cta_to_form_state(c)),
        SectionComponent::Banner(b) => Some(banner_to_form_state(b)),
        SectionComponent::Image(i) => Some(image_to_form_state(i)),
        SectionComponent::HeaderSearch(h) => Some(header_search_to_form_state(h)),
        SectionComponent::HeaderMenu(h) => Some(header_menu_to_form_state(h)),
        SectionComponent::RichText(r) => Some(rich_text_to_form_state(r)),
        SectionComponent::Alert(a) => Some(alert_to_form_state(a)),
        SectionComponent::Modal(m) => Some(modal_to_form_state(m)),
        SectionComponent::Blockquote(bq) => Some(blockquote_to_form_state(bq)),
        _ => None,
    }
}

/// Seed an `EditFormState` with current values from a `DdCta`.
pub fn cta_to_form_state(cta: &DdCta) -> EditFormState {
    let mut state = EditFormState::new(&crate::tui::editform::CTA_FORM);
    state.set("parent_class", enum_serde_str(cta.parent_class));
    state.set("parent_image_url", cta.parent_image_url.clone());
    state.set("parent_image_alt", cta.parent_image_alt.clone());
    state.set("parent_data_aos", enum_serde_str(cta.parent_data_aos));
    state.set("parent_title", cta.parent_title.clone());
    state.set("parent_subtitle", cta.parent_subtitle.clone());
    state.set("parent_copy", cta.parent_copy.clone());
    state.set(
        "parent_link_url",
        cta.parent_link_url.clone().unwrap_or_default(),
    );
    state.set(
        "parent_link_target",
        cta.parent_link_target
            .map(enum_serde_str)
            .unwrap_or_else(|| "_self".to_string()),
    );
    state.set(
        "parent_link_label",
        cta.parent_link_label.clone().unwrap_or_default(),
    );
    state
}

// ==================== Tier A populate + apply ====================

pub fn banner_to_form_state(b: &DdBanner) -> EditFormState {
    let mut s = EditFormState::new(&editform::BANNER_FORM);
    s.set("parent_class", enum_serde_str(b.parent_class));
    s.set("parent_data_aos", enum_serde_str(b.parent_data_aos));
    s.set("parent_image_url", b.parent_image_url.clone());
    s.set("parent_image_alt", b.parent_image_alt.clone());
    s
}
fn apply_banner_values(b: &mut DdBanner, state: &EditFormState) -> Result<()> {
    b.parent_class =
        parse_enum::<BannerClass>(state.get("parent_class")).context("invalid parent_class")?;
    b.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    b.parent_image_url = state.get("parent_image_url").trim().to_string();
    b.parent_image_alt = state.get("parent_image_alt").trim().to_string();
    Ok(())
}

pub fn image_to_form_state(i: &DdImage) -> EditFormState {
    let mut s = EditFormState::new(&editform::IMAGE_FORM);
    s.set("parent_data_aos", enum_serde_str(i.parent_data_aos));
    s.set("parent_image_url", i.parent_image_url.clone());
    s.set("parent_image_alt", i.parent_image_alt.clone());
    s.set(
        "parent_link_url",
        i.parent_link_url.clone().unwrap_or_default(),
    );
    s.set(
        "parent_link_target",
        i.parent_link_target
            .map(enum_serde_str)
            .unwrap_or_else(|| "_self".to_string()),
    );
    s
}
fn apply_image_values(i: &mut DdImage, state: &EditFormState) -> Result<()> {
    i.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    i.parent_image_url = state.get("parent_image_url").trim().to_string();
    i.parent_image_alt = state.get("parent_image_alt").trim().to_string();
    let link = state.get("parent_link_url").trim().to_string();
    if link.is_empty() {
        i.parent_link_url = None;
        i.parent_link_target = None;
    } else {
        i.parent_link_url = Some(link);
        i.parent_link_target = Some(
            parse_enum::<CardLinkTarget>(state.get("parent_link_target"))
                .context("invalid parent_link_target")?,
        );
    }
    Ok(())
}

pub fn header_search_to_form_state(h: &DdHeaderSearch) -> EditFormState {
    let mut s = EditFormState::new(&editform::HEADER_SEARCH_FORM);
    s.set("parent_width", h.parent_width.clone());
    s.set("parent_data_aos", enum_serde_str(h.parent_data_aos));
    s
}
fn apply_header_search_values(h: &mut DdHeaderSearch, state: &EditFormState) -> Result<()> {
    h.parent_width = state.get("parent_width").trim().to_string();
    h.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    Ok(())
}

pub fn header_menu_to_form_state(h: &DdHeaderMenu) -> EditFormState {
    let mut s = EditFormState::new(&editform::HEADER_MENU_FORM);
    s.set("parent_width", h.parent_width.clone());
    s.set("parent_data_aos", enum_serde_str(h.parent_data_aos));
    s
}
fn apply_header_menu_values(h: &mut DdHeaderMenu, state: &EditFormState) -> Result<()> {
    h.parent_width = state.get("parent_width").trim().to_string();
    h.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    Ok(())
}

pub fn rich_text_to_form_state(r: &DdRichText) -> EditFormState {
    let mut s = EditFormState::new(&editform::RICH_TEXT_FORM);
    s.set(
        "parent_class",
        r.parent_class.clone().unwrap_or_default(),
    );
    s.set("parent_data_aos", enum_serde_str(r.parent_data_aos));
    s.set("parent_copy", r.parent_copy.clone());
    s
}
fn apply_rich_text_values(r: &mut DdRichText, state: &EditFormState) -> Result<()> {
    let class = state.get("parent_class").trim().to_string();
    r.parent_class = if class.is_empty() { None } else { Some(class) };
    r.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    r.parent_copy = state.get("parent_copy").to_string();
    Ok(())
}

pub fn alert_to_form_state(a: &DdAlert) -> EditFormState {
    let mut s = EditFormState::new(&editform::ALERT_FORM);
    s.set("parent_type", enum_serde_str(a.parent_type));
    s.set("parent_class", enum_serde_str(a.parent_class));
    s.set("parent_data_aos", enum_serde_str(a.parent_data_aos));
    s.set(
        "parent_title",
        a.parent_title.clone().unwrap_or_default(),
    );
    s.set("parent_copy", a.parent_copy.clone());
    s
}
fn apply_alert_values(a: &mut DdAlert, state: &EditFormState) -> Result<()> {
    a.parent_type =
        parse_enum::<AlertType>(state.get("parent_type")).context("invalid parent_type")?;
    a.parent_class =
        parse_enum::<AlertClass>(state.get("parent_class")).context("invalid parent_class")?;
    a.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    let title = state.get("parent_title").trim().to_string();
    a.parent_title = if title.is_empty() { None } else { Some(title) };
    a.parent_copy = state.get("parent_copy").to_string();
    Ok(())
}

pub fn modal_to_form_state(m: &DdModal) -> EditFormState {
    let mut s = EditFormState::new(&editform::MODAL_FORM);
    s.set("parent_title", m.parent_title.clone());
    s.set("parent_copy", m.parent_copy.clone());
    s
}
fn apply_modal_values(m: &mut DdModal, state: &EditFormState) -> Result<()> {
    m.parent_title = state.get("parent_title").to_string();
    m.parent_copy = state.get("parent_copy").to_string();
    Ok(())
}

pub fn blockquote_to_form_state(bq: &DdBlockquote) -> EditFormState {
    let mut s = EditFormState::new(&editform::BLOCKQUOTE_FORM);
    s.set("parent_data_aos", enum_serde_str(bq.parent_data_aos));
    s.set("parent_image_url", bq.parent_image_url.clone());
    s.set("parent_image_alt", bq.parent_image_alt.clone());
    s.set("parent_name", bq.parent_name.clone());
    s.set("parent_role", bq.parent_role.clone());
    s.set("parent_copy", bq.parent_copy.clone());
    s
}
fn apply_blockquote_values(bq: &mut DdBlockquote, state: &EditFormState) -> Result<()> {
    bq.parent_data_aos = parse_enum::<HeroAos>(state.get("parent_data_aos"))
        .context("invalid parent_data_aos")?;
    bq.parent_image_url = state.get("parent_image_url").trim().to_string();
    bq.parent_image_alt = state.get("parent_image_alt").trim().to_string();
    bq.parent_name = state.get("parent_name").to_string();
    bq.parent_role = state.get("parent_role").to_string();
    bq.parent_copy = state.get("parent_copy").to_string();
    Ok(())
}

/// Serialize a serde enum to its `#[serde(rename = ...)]` string form.
fn enum_serde_str<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(&value)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// Parse a serde enum from its wire string form.
fn parse_enum<T: for<'de> serde::Deserialize<'de>>(input: &str) -> Result<T> {
    serde_json::from_value::<T>(serde_json::Value::String(input.to_string()))
        .map_err(|e| anyhow!("failed to parse '{}': {}", input, e))
}

/// Remove the last `items` index from a cursor — used to navigate out of a
/// nested navigation item when editing its parent. Returns false if already
/// at component level.
#[allow(dead_code)] // exercised during Tier D (dd-navigation)
pub fn pop_items(cursor: &mut Cursor) -> bool {
    let items = match cursor {
        Cursor::HeaderComponent { items, .. }
        | Cursor::FooterComponent { items, .. }
        | Cursor::PageComponent { items, .. } => items,
        _ => return false,
    };
    items.pop().is_some()
}

/// Skip-counts FormField writes using the form's own declared kind. Returns
/// the number of values the form would attempt to round-trip — used by tests
/// to assert completeness of the wedge form wiring.
#[allow(dead_code)]
pub fn form_field_value_count(state: &EditFormState) -> usize {
    let mut count = 0usize;
    for field in state.form.fields {
        if !state.field_visible(field) {
            continue;
        }
        count += match &field.kind {
            FieldKind::OptionalLinkTriple { .. } => 3,
            _ => 1,
        };
    }
    count
}
