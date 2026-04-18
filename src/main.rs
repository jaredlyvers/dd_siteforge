mod model;
mod renderer;
mod storage;
mod tui;
mod validate;

use anyhow::Context;
use clap::{Parser, Subcommand};
use model::Site;
use renderer::render_site_to_dir;
use std::path::PathBuf;
use storage::{load_site, save_site};
use tui::run_tui;
use validate::validate_site;

#[derive(Debug, Parser)]
#[command(name = "dd_staticsite")]
#[command(about = "Framework-native static page builder")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    InitSite { path: String },
    ShowSite { path: String },
    ValidateSite { path: String },
    ExportHtml { input: String, output_dir: String },
    Tui { path: Option<String> },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::InitSite { path } => {
            let site = Site::starter();
            save_site(&path, &site)
                .with_context(|| format!("could not write starter site to '{}'", path))?;
            println!("Created starter site at {}", path);
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
            let errors = validate_site(&site);
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
            let errors = validate_site(&site);
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
            render_site_to_dir(&site, &out_path).with_context(|| {
                format!(
                    "could not export site '{}' to '{}'",
                    input,
                    out_path.display()
                )
            })?;
            println!("Exported HTML to {}", out_path.display());
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
