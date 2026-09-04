//! Export and preview path prompts plus local HTTP preview.
use super::super::*;

impl App {
    pub(in crate::tui) fn handle_export_path_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let path = if let Some(Modal::ExportPathPrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Export cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_export_path_from_prompt(path),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_export_path_from_prompt(path)
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::ExportPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::ExportPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::ExportPathPrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }

    pub(in crate::tui) fn handle_preview_path_prompt_event(&mut self, key: event::KeyEvent) -> Option<ModalResult> {
        use crossterm::event::KeyCode;
        let path = if let Some(Modal::PreviewPathPrompt { path }) = self.modal.take() {
            path
        } else {
            return Some(ModalResult::CloseCancel);
        };
        match key.code {
            KeyCode::Esc => {
                self.push_toast(ToastLevel::Info, "Preview cancelled.");
                Some(ModalResult::CloseCancel)
            }
            KeyCode::Enter => self.commit_preview_path_from_prompt(path),
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.commit_preview_path_from_prompt(path)
            }
            KeyCode::Backspace => {
                let mut new_path = path;
                new_path.pop();
                self.modal = Some(Modal::PreviewPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut new_path = path;
                new_path.push(c);
                self.modal = Some(Modal::PreviewPathPrompt { path: new_path });
                Some(ModalResult::Continue)
            }
            _ => {
                self.modal = Some(Modal::PreviewPathPrompt { path });
                Some(ModalResult::Continue)
            }
        }
    }

    pub(in crate::tui) fn commit_preview_path_from_prompt(&mut self, path: String) -> Option<ModalResult> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Preview path required.");
            self.modal = Some(Modal::PreviewPathPrompt { path });
            Some(ModalResult::Continue)
        } else {
            self.commit_preview_to(trimmed.to_string());
            Some(ModalResult::CloseSuccess)
        }
    }

    pub(in crate::tui) fn commit_preview_to(&mut self, rel: String) {
        use std::path::{Path, PathBuf};
        let normalized = normalize_relative_path(&rel);
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let out = base.join(Path::new(&normalized));

        if let Err(e) = crate::export::export_site(&self.site, &out, Some(&base)) {
            let msg = format!("Preview failed: {}", e);
            self.push_toast(ToastLevel::Error, msg);
            return;
        }
        self.site.export_dir = Some(normalized.clone());

        let display = display_relative_path(&base, &out, &normalized);
        let count = self.site.pages.len();
        self.push_toast(
            ToastLevel::Success,
            format!("Exported {} page(s) to {}", count, display),
        );
        match self.ensure_preview_server(out.clone()) {
            Ok(url) => match open_in_browser(&url) {
                Ok(()) => {
                    self.push_toast(
                        ToastLevel::Info,
                        format!("Opening {} in browser…", url),
                    );
                }
                Err(e) => {
                    self.push_toast(
                        ToastLevel::Error,
                        format!("Browser open failed: {}", e),
                    );
                }
            },
            Err(e) => {
                self.push_toast(
                    ToastLevel::Error,
                    format!("Preview server failed: {}", e),
                );
            }
        }
    }

    pub(in crate::tui) fn ensure_preview_server(&mut self, out: PathBuf) -> anyhow::Result<String> {
        let slug = self.current_page_slug_for_preview();
        if let Some(server) = self.preview_server.as_ref() {
            server.set_root(out);
            return Ok(server.url_for(&slug));
        }
        let server = crate::serve::StaticServer::start(out)?;
        let url = server.url_for(&slug);
        self.preview_server = Some(server);
        Ok(url)
    }

    pub(in crate::tui) fn current_page_slug_for_preview(&self) -> String {
        let idx = self.selected_page.min(self.site.pages.len().saturating_sub(1));
        self.site
            .pages
            .get(idx)
            .map(|p| p.slug.clone())
            .unwrap_or_else(|| "index".to_string())
    }

    pub(in crate::tui) fn begin_preview_flow(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if !errors.is_empty() {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
            return;
        }
        match self.site.export_dir.clone() {
            Some(dir) if !dir.trim().is_empty() => {
                self.commit_preview_to(dir);
            }
            _ => {
                self.modal = Some(Modal::PreviewPathPrompt {
                    path: "./web/".to_string(),
                });
            }
        }
    }

    pub(in crate::tui) fn begin_export_flow(&mut self) {
        let root = self.path.as_ref().and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        let errors = crate::validate::validate_site_with_root(&self.site, root.as_deref());
        if !errors.is_empty() {
            self.modal = Some(Modal::ValidationErrors {
                errors,
                scroll_offset: 0,
            });
            return;
        }
        match self.site.export_dir.clone() {
            Some(dir) if !dir.trim().is_empty() => {
                self.commit_export_to(dir);
            }
            _ => {
                self.modal = Some(Modal::ExportPathPrompt {
                    path: "./web/".to_string(),
                });
            }
        }
    }

    pub(in crate::tui) fn commit_export_path_from_prompt(&mut self, path: String) -> Option<ModalResult> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            self.push_toast(ToastLevel::Warning, "Export path required.");
            self.modal = Some(Modal::ExportPathPrompt { path });
            Some(ModalResult::Continue)
        } else {
            self.commit_export_to(trimmed.to_string());
            Some(ModalResult::CloseSuccess)
        }
    }

    pub(in crate::tui) fn commit_export_to(&mut self, rel: String) {
        use std::path::{Path, PathBuf};
        let normalized = normalize_relative_path(&rel);
        let base = self
            .path
            .as_ref()
            .and_then(|p| p.parent().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."));
        let out = base.join(Path::new(&normalized));

        match crate::export::export_site(&self.site, &out, Some(&base)) {
            Ok(report) => {
                self.site.export_dir = Some(normalized.clone());
                let display = display_relative_path(&base, &out, &normalized);
                let msg = if report.wrote_404 {
                    format!("Exported {} page(s) to {} (wrote 404.html)", report.pages, display)
                } else {
                    format!("Exported {} page(s) to {}", report.pages, display)
                };
                self.push_toast(ToastLevel::Success, msg);
            }
            Err(e) => {
                let msg = format!("Export failed: {}", e);
                self.push_toast(ToastLevel::Error, msg);
            }
        }
    }
}
