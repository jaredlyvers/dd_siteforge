mod export;
mod model;
mod renderer;
mod scaffold;
mod serve;
mod storage;
mod templates;
mod tui;
mod validate;

use anyhow::Context;
use clap::{Parser, Subcommand};
use export::export_site;
use model::Site;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
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
    /// Create a starter site.json and seed the build kit (source/, Grunt, Lando/DDEV).
    InitSite {
        path: String,
        /// Project slug for Lando, DDEV, and package.json (skips the prompt).
        #[arg(long)]
        name: Option<String>,
    },
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
    /// Seed Grunt / source / Lando / DDEV (skips files that already exist).
    InitScaffold {
        /// Site JSON path; kit goes next to it. Required unless --global.
        path: Option<String>,
        /// Overwrite existing scaffold files.
        #[arg(long)]
        force: bool,
        /// Write the bundled kit into ~/.config/ldnddev/dd_siteforge/ (no name stamp).
        #[arg(long)]
        global: bool,
        /// Project slug for Lando, DDEV, and package.json (default: folder name).
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
        Command::InitSite { path, name } => {
            let site = Site::starter();
            save_site(&path, &site)
                .with_context(|| format!("could not write starter site to '{}'", path))?;
            let root = site_root(&path);
            fs::create_dir_all(&root)
                .with_context(|| format!("could not create '{}'", root.display()))?;
            let slug = resolve_project_name(name.as_deref(), &root)?;
            let overlay = scaffold::overlay_if_present();
            let kit = scaffold::seed_scaffold(
                &root,
                scaffold::SeedOpts {
                    force: false,
                    project_name: Some(&slug),
                    overlay: overlay.as_deref(),
                },
            )?;
            let seeded = templates::seed_templates(&root, false, None)?;
            println!("Created starter site at {}", path);
            println!("Project name: {slug}");
            print_seed_report(&kit);
            if !seeded.written.is_empty() {
                println!(
                    "Wrote {} template(s) to {}/source/templates/",
                    seeded.written.len(),
                    root.display()
                );
            }
            if !seeded.skipped.is_empty() {
                println!(
                    "Skipped {} existing template(s) (use init-templates --force to overwrite)",
                    seeded.skipped.len()
                );
            }
        }
        Command::InitTemplates { path, force, name } => {
            let root = site_root(&path);
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
        Command::InitScaffold {
            path,
            force,
            global,
            name,
        } => {
            if global {
                let dest = scaffold::config_scaffold_dir();
                let report = scaffold::seed_scaffold(
                    &dest,
                    scaffold::SeedOpts {
                        force,
                        project_name: None,
                        overlay: None,
                    },
                )?;
                println!("Global scaffold: {}", dest.display());
                print_seed_report(&report);
            } else {
                let path = path.ok_or_else(|| {
                    anyhow::anyhow!("path is required unless --global is set")
                })?;
                let root = site_root(&path);
                let slug = name
                    .as_deref()
                    .map(scaffold::slugify_project_name)
                    .unwrap_or_else(|| {
                        scaffold::slugify_project_name(&scaffold::dir_hint(&root))
                    });
                let overlay = scaffold::overlay_if_present();
                let report = scaffold::seed_scaffold(
                    &root,
                    scaffold::SeedOpts {
                        force,
                        project_name: Some(&slug),
                        overlay: overlay.as_deref(),
                    },
                )?;
                println!("Project name: {slug}");
                print_seed_report(&report);
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

fn site_root(path: &str) -> PathBuf {
    PathBuf::from(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resolve_project_name(flag: Option<&str>, root: &Path) -> anyhow::Result<String> {
    if let Some(raw) = flag {
        return Ok(scaffold::slugify_project_name(raw));
    }
    let default = scaffold::slugify_project_name(&scaffold::dir_hint(root));
    let interactive = io::stdin().is_terminal() && io::stderr().is_terminal();
    if !interactive {
        return Ok(default);
    }
    eprint!("Project name [{default}]: ");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        Ok(default)
    } else {
        Ok(scaffold::slugify_project_name(trimmed))
    }
}

fn print_seed_counts(label: &str, report: &scaffold::SeedReport) {
    if !report.written.is_empty() {
        println!("Wrote {} {label}", report.written.len());
    }
}

fn print_seed_report(report: &scaffold::SeedReport) {
    print_seed_counts("scaffold file(s)", report);
    if !report.skipped.is_empty() {
        println!(
            "Skipped {} existing (use --force to overwrite)",
            report.skipped.len()
        );
    }
}
