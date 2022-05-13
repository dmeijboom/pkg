use anyhow::{anyhow, Result};
use clap::{Parser as ClapParser, ValueHint};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::installer::{InstallOpts, Installer};
use crate::package::Package;
use crate::store::{list_installed, Action, Store, Transaction};
use crate::utils::root_dir;

#[derive(ClapParser, Debug)]
pub struct Opts {
    #[clap(value_hint = ValueHint::FilePath)]
    filename: PathBuf,
    #[clap(long)]
    force: bool,
    #[clap(long)]
    no_publish: bool,
}

fn parse_package_config(filename: PathBuf) -> Result<Package> {
    let content = fs::read_to_string(filename)?;
    let package = serde_dhall::from_str(&content).imports(true).parse()?;

    Ok(package)
}

pub fn run(opts: Opts) -> Result<()> {
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

    let installer = Installer::new(package);

    installer.install(
        root,
        InstallOpts {
            force: opts.force,
            publish: !opts.no_publish,
        },
    )?;

    store.add(&Transaction::new(
        store.root()?,
        package_id,
        Action::Install,
    ))?;

    println!("{}", ">> installed".blue());

    Ok(())
}
