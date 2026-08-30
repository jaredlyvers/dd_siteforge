//! Layout tree navigation and structural edits.
mod build;
mod expand;
mod open;
mod nav;
mod edit;
mod items;
mod columns;

#[derive(Clone, Copy)]
pub(in crate::tui) struct TreeRow {
    pub(in crate::tui) kind: TreeRowKind,
}

#[derive(Clone, Copy)]
pub(in crate::tui) enum TreeRowKind {
    HeaderRoot,
    HeaderSection {
        section_idx: usize,
    },
    HeaderColumn {
        section_idx: usize,
        column_idx: usize,
    },
    HeaderComponent {
        section_idx: usize,
        column_idx: usize,
        component_idx: usize,
    },
    FooterRoot,
    FooterSection {
        section_idx: usize,
    },
    FooterColumn {
        section_idx: usize,
        column_idx: usize,
    },
    FooterComponent {
        section_idx: usize,
        column_idx: usize,
        component_idx: usize,
    },
    PageHead,
    Hero {
        node_idx: usize,
    },
    Section {
        node_idx: usize,
    },
    Column {
        node_idx: usize,
        column_idx: usize,
    },
    Component {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
    },
    AccordionItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    AlternatingItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    CardItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    FilmstripItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    MilestonesItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
    SliderItem {
        node_idx: usize,
        column_idx: usize,
        component_idx: usize,
        item_idx: usize,
    },
}

