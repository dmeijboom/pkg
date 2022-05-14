use crate::installer::{InstallOpts, Installer, Stage};
use crate::utils::{parse_package_config, root_dir};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use std::process::exit;

#[derive(Debug, Parser)]
pub struct Opts {
    filename: PathBuf,
}

pub fn run(opts: Opts) -> Result<()> {
    let root = root_dir();
    let package = parse_package_config(opts.filename)?;
    let package_id = format!("{}@{}", package.name, package.version);

    println!("{}", format!(">> validating {}", package_id).blue());

    let mut ok = true;

    for os in package.sources.keys() {
        if let Some((os, architectures)) = package.sources.get(os).and_then(|t| {
            Some((
                os,
                t.keys().into_iter().filter(|arch| t.get(arch).is_some()),
            ))
        }) {
            for arch in architectures {
                println!(
                    "{}",
                    format!(">> validating sources for target {}_{}... ", os, arch).blue()
                );

                let installer = Installer::new(&package);

                if let Err(e) = installer.install(
                    root.clone(),
                    InstallOpts {
                        os,
                        arch,
                        force: false,
                        stage: Stage::FetchSources,
                    },
                ) {
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
