### Architecture

**Overview:**  
Terminal User Interface (TUI) app for building HTML pages via a CMS-like workflow. Built in Rust using ratatui for UI (with mouse support enabled), serde for data handling, and handlebars for HTML templating. Data model: Hierarchical structure with Site > Pages > Sections > Columns > Components. Output: Static HTML files with flex-based CSS grid.

**Key Components:**  
- **UI Layer:** Ratatui widgets for menus, lists, forms (e.g., selectable lists for sections/columns/components). Enable mouse events via crossterm: capture clicks for selection, drags for resizing (if applicable), scrolls for navigation.  
- **Data Model:** Structs for Section (id, columns: Vec<Column>), Column (width_class: String, components: Vec<Component>), Component (type: Enum e.g., Text/Image/Link, fields: HashMap<String, String>).  
- **Logic Layer:** Commands for add/edit/generate; flex grid classes (e.g., "flex-1", "flex-2" from predefined set). Handle mouse inputs in event loop.  
- **Output Generator:** Serialize data to HTML via templates, embedding images/links/copy.  
- **Storage:** JSON files for site state persistence.

**Tech Stack:**  
- Rust core.  
- Dependencies: ratatui, crossterm (input with mouse enabled), serde_json (storage), handlebars (templating), image (optional for previews).

### Build Plan

**Phase 1: Setup & Core UI (MVP Skeleton)**  
- Initialize Rust project with dependencies. Enable mouse in crossterm backend.  
- Build basic TUI: Main menu, navigation (keys: arrow/enter/esc; mouse: clicks for select, wheel for scroll).  
- Implement data model structs.  
- Checkpoint: Run app, navigate empty UI with mouse/keyboard without crashes.

**Phase 2: Section & Column Management**  
- Add commands: Create/delete sections.  
- Per section: Add/delete columns, select flex grid widths (e.g., menu with options like 1/12, 2/12; mouse-click to choose).  
- UI: List views for sections/columns, mouse-selectable.  
- Checkpoint: Create a page with sections and columns, view structure in TUI using mouse.

**Phase 3: Component Integration**  
- Define component list (e.g., Text, Image, Link, Button).  
- Commands: Add component to column, edit fields (prompts for copy/url/alt-text; mouse for form navigation).  
- UI: Sub-menus for component selection/editing, mouse-clickable.  
- Checkpoint: Add/edit components in columns, verify data model updates with mouse.

**Phase 4: HTML Generation & Export**  
- Implement templating: Generate HTML with flex classes, embed component content.  
- Commands: Preview (text-based), export to file.  
- Handle assets: Copy images to output dir if paths provided.  
- Checkpoint: Generate and view sample HTML file from built page.

**Phase 5: Full CMS Features & Polish**  
- Site-level: Multiple pages, global settings (e.g., theme).  
- Persistence: Load/save site JSON.  
- Error handling, help menu, undo. Mouse-specific: Tooltips on hover if supported.  
- Testing: Unit tests for data/logic, manual TUI runs with mouse.  
- Checkpoint: Build multi-page site, export complete static site using mouse controls.

**Phase 6: Deployment & Extensions**  
- Package as CLI tool (cargo install).  
- Optional: Integrate with SSG like Hugo/Zola for full static site.  
- Documentation: README with usage, including mouse controls.  
- Checkpoint: Install and run end-to-end build/export with mouse.