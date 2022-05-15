use std::path::PathBuf;
use std::process::exit;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use crate::install::channel::Receiver;
use crate::install::{self, Event, Installer, Stage};
use crate::utils::{parse_package_config, root_dir};

#[derive(Parser)]
pub struct Opts {
    filename: PathBuf,
}

pub async fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let package = parse_package_config(opts.filename)?;
    let package_id = format!("{}@{}", package.name, package.version);

    println!("{}", format!(">> validating {}", package_id).blue());

    let mut ok = true;

    for os in package.sources.keys() {
        if let Some((os, architectures)) = package.sources.get(os).map(|t| (os, t.valid_keys())) {
            for arch in architectures {
                println!(
                    "{}",
                    format!(">> validating sources for target {}_{}... ", os, arch).blue()
                );

                let (installer, rx) = Installer::new(&package, root.clone())?;
                let progress = tokio::spawn(async move { show_progress(rx).await });

                let result = installer
                    .install(install::Opts {
                        os,
                        arch,
                        force: false,
                        stage: Stage::FetchSources,
                    })
                    .await;

                progress.await?;

                if let Err(e) = result {
                    ok = false;
                    eprintln!("{}", format!("{}", e).red());
                }
            }
        }
    }

    if ok {
        println!("{}", ">> validation succeeded".green());
    } else {
        eprintln!("{}", ">> validation failed".red());
        exit(1);
    }

    Ok(())
}

async fn show_progress(mut rx: Receiver) {
    while let Some(event) = rx.recv().await {
        match event {
            Event::EnterStage(stage) => {
                println!(
                    "{}",
                    format!(
                        "# {}",
                        match stage {
                            Stage::FetchSources => "fetching sources",
                            Stage::EvalPkgscript => "evaluating pkgscript",
                            Stage::Package => "packaging",
                            Stage::Publish => "publishing",
                        }
                    )
                    .white()
                    .bold()
                );
            }
            Event::ExitStage(_) => {}
            Event::Message(_, msg) => {
                println!("{}", msg.white());
            }
        }
    }
}
