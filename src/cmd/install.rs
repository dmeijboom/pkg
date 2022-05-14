use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser as ClapParser, ValueHint};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::install::channel::Receiver;
use crate::install::{self, Event, Installer, Stage};
use crate::store::{list_installed, Action, Store, Transaction};
use crate::utils::{parse_package_config, root_dir};

#[derive(ClapParser, Debug)]
pub struct Opts {
    #[clap(value_hint = ValueHint::FilePath)]
    filename: PathBuf,
    #[clap(long)]
    force: bool,
    #[clap(long)]
    no_publish: bool,
}

pub async fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let store = Store::new(root.join("store"));
    let package = parse_package_config(opts.filename)?;
    let package_id = format!("{}@{}", package.name, package.version);

    if !opts.force
        && list_installed(&store)?
            .iter()
            .any(|tx| tx.package_id == package_id)
    {
        return Err(anyhow!("package is already installed"));
    }

    println!("{}", format!(">> installing {}", package_id).blue());

    let total_stages = if opts.no_publish { 3 } else { 4 };
    let (installer, rx) = Installer::new(&package, root)?;
    let progress = tokio::spawn(async move { show_progress(total_stages, rx).await });

    installer
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

    store.add(&Transaction::new(
        store.root()?,
        package_id,
        Action::Install,
    ))?;

    println!("{}", ">> installed".blue());

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
