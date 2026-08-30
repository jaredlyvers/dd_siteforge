mod export;
mod model;
mod renderer;
mod serve;
mod storage;
mod templates;
mod tui;
mod validate;

use anyhow::Context;
use clap::{Parser, Subcommand};
use export::export_site;
use model::Site;
use std::path::PathBuf;
use storage::{load_site, save_site};
use tui::run_tui;
use validate::validate_site_with_root;

#[derive(Debug, Parser)]
#[command(name = "dd_siteforge", version, about = "Framework-native static site builder")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    InitSite { path: String },
    /// Write default Handlebars templates into source/templates (skips files that already exist).
    InitTemplates {
        /// Site JSON path; templates go next to it in source/templates/.
        path: String,
        /// Overwrite existing template files.
        #[arg(long)]
        force: bool,
        /// Seed only this template (e.g. dd-hero).
        #[arg(long)]
        name: Option<String>,
    },
    ShowSite { path: String },
    ValidateSite { path: String },
    ExportHtml { input: String, output_dir: String },
    /// Export then serve the site over HTTP for local preview.
    Serve {
        path: String,
        #[arg(long, default_value_t = 8765)]
        port: u16,
        #[arg(long)]
        output_dir: Option<String>,
    },
    Tui { path: Option<String> },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::InitSite { path } => {
            let site = Site::starter();
            save_site(&path, &site)
                .with_context(|| format!("could not write starter site to '{}'", path))?;
            let root = PathBuf::from(&path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let seeded = templates::seed_templates(&root, false, None)?;
            println!("Created starter site at {}", path);
            if !seeded.written.is_empty() {
                println!(
                    "Wrote {} template(s) to {}/source/templates/",
                    seeded.written.len(),
                    root.display()
                );
            }
        }
        Command::InitTemplates { path, force, name } => {
            let root = PathBuf::from(&path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let report = templates::seed_templates(&root, force, name.as_deref())?;
            if !report.written.is_empty() {
                println!("Wrote: {}", report.written.join(", "));
            }
            if !report.skipped.is_empty() {
                println!(
                    "Skipped existing (use --force to overwrite): {}",
                    report.skipped.join(", ")
                );
            }
        }
        Command::ShowSite { path } => {
            let site =
                load_site(&path).with_context(|| format!("could not load site '{}'", path))?;
            let json = serde_json::to_string_pretty(&site)?;
            println!("{json}");
        }
        Command::ValidateSite { path } => {
            let site =
                load_site(&path).with_context(|| format!("could not load site '{}'", path))?;
            let root = PathBuf::from(&path)
                .parent()
                .map(PathBuf::from);
            let errors = validate_site_with_root(&site, root.as_deref());
            if errors.is_empty() {
                println!("Validation passed.");
            } else {
                println!("Validation failed with {} error(s):", errors.len());
                for err in errors {
                    println!("- {}", err);
                }
                std::process::exit(1);
            }
        }
        Command::ExportHtml { input, output_dir } => {
            let site =
                load_site(&input).with_context(|| format!("could not load site '{}'", input))?;
            let root = PathBuf::from(&input)
                .parent()
                .map(PathBuf::from);
            let errors = validate_site_with_root(&site, root.as_deref());
            if !errors.is_empty() {
                println!(
                    "Refusing export: validation failed with {} error(s):",
                    errors.len()
                );
                for err in errors {
                    println!("- {}", err);
                }
                std::process::exit(1);
            }
            let out_path = PathBuf::from(&output_dir);
            let report = export_site(&site, &out_path, root.as_deref()).with_context(|| {
                format!(
                    "could not export site '{}' to '{}'",
                    input,
                    out_path.display()
                )
            })?;
            println!(
                "Exported {} page(s) to {}{}",
                report.pages,
                out_path.display(),
                if report.wrote_404 {
                    " (wrote 404.html)"
                } else {
                    ""
                }
            );
        }
        Command::Serve {
            path,
            port,
            output_dir,
        } => {
            let site =
                load_site(&path).with_context(|| format!("could not load site '{}'", path))?;
            let root = PathBuf::from(&path)
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let errors = validate_site_with_root(&site, Some(&root));
            if !errors.is_empty() {
                println!(
                    "Refusing serve: validation failed with {} error(s):",
                    errors.len()
                );
                for err in errors {
                    println!("- {}", err);
                }
                std::process::exit(1);
            }
            let out = output_dir
                .or_else(|| site.export_dir.clone())
                .unwrap_or_else(|| "web".to_string());
            let out_path = root.join(&out);
            let report = export_site(&site, &out_path, Some(&root)).with_context(|| {
                format!("could not export site '{}' to '{}'", path, out_path.display())
            })?;
            println!(
                "Exported {} page(s) to {}{}",
                report.pages,
                out_path.display(),
                if report.wrote_404 {
                    " (wrote 404.html)"
                } else {
                    ""
                }
            );
            serve::serve_dir_blocking(out_path, port)?;
        }
        Command::Tui { path } => {
            let loaded = if let Some(p) = path.as_ref() {
                load_site(p).with_context(|| format!("could not load site '{}'", p))?
            } else {
                Site::starter()
            };
            let path_buf = path.map(PathBuf::from);
            run_tui(loaded, path_buf)?;
        }
    }

    Ok(())
}
