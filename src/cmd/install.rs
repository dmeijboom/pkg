use std::fs::Permissions;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{anyhow, Context, Result};
use clap::{Parser as ClapParser, ValueHint};
use colored::Colorize;
use fs_extra::dir::{move_dir, CopyOptions};
use globset::Glob;
use temp_dir::TempDir;

use crate::download::download_and_unpack;
use crate::package::Package;
use crate::pkgscript::{Instruction, Parser};
use crate::store::{list_installed, Action, Store, Transaction};
use crate::utils::root_dir;

#[derive(ClapParser, Debug)]
pub struct Opts {
    #[clap(value_hint = ValueHint::FilePath)]
    filename: PathBuf,
    #[clap(long)]
    force: bool,
}

fn set_env_vars() {
    env::set_var("TARGET_OS", env::consts::OS);
    env::set_var("TARGET_ARCH", env::consts::ARCH);
    env::set_var(
        "TARGET_VENDOR",
        match env::consts::OS {
            "macos" => "apple",
            "windows" => "pc",
            _ => "unknown",
        },
    );
    env::set_var("TARGET_FAMILY", env::consts::FAMILY);
}

fn parse_package_config(filename: PathBuf) -> Result<Package> {
    let content = fs::read_to_string(filename)?;
    let package = serde_dhall::from_str(&content).imports(true).parse()?;

    Ok(package)
}

fn find_file(dir: impl AsRef<Path>, pat: Glob) -> Result<PathBuf> {
    let matcher = pat.compile_matcher();
    let read_dir = dir.as_ref().read_dir()?;

    for entry in read_dir {
        let entry = entry?;

        if matcher.is_match(entry.path()) {
            return Ok(entry.path());
        }
    }

    Err(anyhow!("no such file found for pattern: {}", pat))
}

pub fn run(opts: Opts) -> Result<()> {
    set_env_vars();

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

    let dir = TempDir::new()?;
    let sources = package
        .sources
        .get(env::consts::OS)
        .and_then(|targets| targets.get(env::consts::ARCH))
        .ok_or_else(|| {
            anyhow!(
                "no sources found for target: {}.{}",
                env::consts::OS,
                env::consts::ARCH
            )
        })?;

    for source in sources {
        println!("{}", format!("downloading {}", source.url).white());

        let checksum = download_and_unpack(&source.url, dir.child("sources"))?;

        if source.checksum != checksum {
            return Err(anyhow!(
                "checksum mismatch for source '{}' (expected: '{}', got: '{}')",
                source.url,
                source.checksum,
                checksum
            ));
        }
    }

    println!("{}", ">> evaluate pkgscript".blue());

    let out_dir = dir.child("output");
    let out_bin_dir = out_dir.join("bin");

    fs::create_dir_all(&out_dir)?;
    fs::create_dir_all(&out_bin_dir)?;

    let script = Parser::parse(&package.install)?;
    let mut published = vec![];

    for instruction in script.body {
        println!("{}", instruction.to_string().white());

        match instruction {
            Instruction::Package { source, target } => {
                let source = if let Some(idx) = source.find('*') {
                    let pattern = format!("{}/{}", dir.path().to_str().unwrap(), source);
                    let glob = Glob::new(&pattern)?;
                    let prefix = dir.child(&source[..idx]);

                    find_file(prefix.parent().unwrap(), glob)?
                } else {
                    dir.child(source)
                };
                let dest = match target {
                    Some(target) => out_bin_dir.join(target),
                    None => out_bin_dir.join(source.file_name().unwrap()),
                };

                fs::copy(&source, &dest).context("copy failed")?;
                fs::set_permissions(dest, Permissions::from_mode(0o755))?;
            }
            Instruction::Publish { target } => {
                let dest = out_bin_dir.join(&target);

                if PathBuf::from(&target).components().count() > 1 {
                    return Err(anyhow!("publish target must contain only the filename"));
                }

                if !dest.exists() {
                    return Err(anyhow!("unable to publish unknown target: {}", target));
                }

                published.push(target);
            }
        }
    }

    println!("{}", ">> packaging".blue());

    let packages_dir = root.join("packages").join(package.name);

    if !packages_dir.exists() {
        fs::create_dir_all(&packages_dir)?;
    }

    let mut copy_opts = CopyOptions::new();

    copy_opts.overwrite = opts.force;
    copy_opts.content_only = true;

    let pkg_dir = packages_dir.join(package.version);

    move_dir(out_dir, &pkg_dir, &copy_opts).context("move to destination failed")?;

    let out_bin_dir = root.join("bin");

    if !out_bin_dir.exists() {
        fs::create_dir_all(&out_bin_dir)?;
    }

    let pkg_bin_dir = pkg_dir.join("bin");

    for target in published {
        symlink(pkg_bin_dir.join(&target), out_bin_dir.join(target))?;
    }

    store.add(&Transaction::new(
        store.root()?,
        package_id,
        Action::Install,
    ))?;

    println!("{}", ">> installed".blue());

    Ok(())
}
