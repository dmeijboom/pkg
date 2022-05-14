use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser as ClapParser, ValueHint};
use colored::Colorize;

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

    let (installer, rx) = Installer::new(&package, root)?;
    let progress = tokio::spawn(async move { show_progress(rx).await });

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

async fn show_progress(mut rx: Receiver) {
    while let Some(event) = rx.recv().await {
        match event {
            Event::EnterStage(stage) => {
                println!(
                    "{}",
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
                );
            }
            Event::ExitStage(_) => {}
            Event::Message(_, msg) => {
                println!("{}", msg.white());
            }
        }
    }
}
