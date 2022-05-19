use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser as ClapParser, ValueHint};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::install::channel::Receiver;
use crate::install::{self, Event, Installer, Stage};
use crate::store::{Storage, Store, Transaction, TransactionKind};
use crate::utils::{read_package_config, root_dir};

#[derive(ClapParser)]
pub struct Opts {
    id: Option<String>,
    #[clap(short, value_hint = ValueHint::FilePath)]
    filename: Option<PathBuf>,
    #[clap(long)]
    force: bool,
    #[clap(long)]
    no_publish: bool,
}

pub async fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let storage = Storage::new(root.join("store"));
    let mut store = Store::new(&storage);
    let package = if let Some(id) = opts.id {
        store
            .search_package(&id)
            .await?
            .ok_or_else(|| anyhow!("package not found"))
    } else if let Some(filename) = opts.filename {
        read_package_config(filename)
    } else {
        return Err(anyhow!("either name or filename must be specified"));
    }?;
    let package_id = format!("{}@{}", package.name, package.version);

    if !opts.force && store.find_installed_package(&package_id).await?.is_some() {
        return Err(anyhow!("package is already installed"));
    }

    println!("{}", format!(">> installing {}", package_id).blue());

    let total_stages = if opts.no_publish { 3 } else { 4 };
    let (installer, rx) = Installer::new(&package, root)?;
    let progress = tokio::spawn(async move { show_progress(total_stages, rx).await });

    let result = installer
        .install(install::Opts {
            os: env::consts::OS,
            arch: env::consts::ARCH,
            force: opts.force,
            stage: if opts.no_publish {
                Stage::Package
            } else {
                Stage::Publish
            },
        })
        .await?;

    progress.await?;

    store
        .add(Transaction::new(TransactionKind::InstallPackage {
            package_id,
            content: result.content,
        }))
        .await?;

    println!("{}", "✓ added".green());

    Ok(())
}

async fn show_progress(total_stages: usize, mut rx: Receiver) {
    let style = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap()
        .progress_chars("##-");
    let pb = ProgressBar::new(total_stages as u64).with_style(style);

    while let Some(event) = rx.recv().await {
        match event {
            Event::EnterStage(stage) => {
                pb.println(
                    format!(
                        ">> {}",
                        match stage {
                            Stage::FetchSources => "fetching sources",
                            Stage::EvalPkgscript => "evaluating pkgscript",
                            Stage::Package => "packaging",
                            Stage::Publish => "publishing",
                        }
                    )
                    .blue()
                    .to_string(),
                );
            }
            Event::ExitStage(_) => {
                pb.inc(1);
            }
            Event::Message(_, msg) => {
                pb.println(msg.white().to_string());
            }
        }

        pb.tick();
    }

    pb.finish_and_clear();
}
