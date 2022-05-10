use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{env, fs};

use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use colored::Colorize;
use fs_extra::dir::{move_dir, CopyOptions};
use temp_dir::TempDir;

use crate::config::Package;
use crate::download::download;
use crate::pkgscript::{Instruction, Parser};
use crate::store::{Action, Store, Transaction};
use crate::utils::root_dir;

#[derive(ClapParser, Debug)]
pub struct Opts {
    filename: PathBuf,
}

fn set_env_vars() {
    env::set_var("TARGET_OS", env::consts::OS);
    env::set_var("TARGET_ARCH", env::consts::ARCH);
    env::set_var("TARGET_FAMILY", env::consts::FAMILY);
}

fn parse_package_config(filename: PathBuf) -> Result<Package> {
    let content = fs::read_to_string(filename)?;
    let package = serde_dhall::from_str(&content)
        .imports(true)
        .static_type_annotation()
        .parse()?;

    Ok(package)
}

pub fn run(opts: Opts) -> Result<()> {
    set_env_vars();

    let package = parse_package_config(opts.filename)?;
    let package_id = format!("{}@{}", package.name, package.version);

    println!("{}", format!(">> installing {}", package_id).blue());

    let dir = TempDir::new()?;

    for source in package.source.iter() {
        println!("{}", format!("downloading '{}'", source).white());

        download(source, dir.child("sources"))?;
    }

    println!("{}", ">> evaluate pkgscript".blue());

    let out_dir = dir.child("output");
    let bin_dir = out_dir.join("bin");

    fs::create_dir_all(&out_dir)?;
    fs::create_dir_all(&bin_dir)?;

    let script = Parser::parse(&package.install)?;

    for instruction in script.body {
        println!("{}", instruction.to_string().white());

        match instruction {
            Instruction::Package { source, target } => {
                let source = dir.child(source);
                let dest = match target {
                    Some(target) => bin_dir.join(target),
                    None => bin_dir.join(source.file_name().unwrap()),
                };

                fs::copy(&source, &dest).context("copy failed")?;
                fs::set_permissions(dest, Permissions::from_mode(0o755))?;
            }
        }
    }

    println!("{}", ">> packaging".blue());

    let root = root_dir();
    let pkg_dir = root.join("packages").join(package.name);

    if !pkg_dir.exists() {
        fs::create_dir_all(&pkg_dir)?;
    }

    let mut opts = CopyOptions::new();

    opts.content_only = true;

    move_dir(out_dir, pkg_dir.join(package.version), &opts)
        .context("move to destination failed")?;

    let store = Store::new(root.join("store"));
    let root = store.root()?;

    store.add(&Transaction::new(root, package_id, Action::Install))?;

    println!("{}", ">> installed".blue());

    Ok(())
}
