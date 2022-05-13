use std::fs::Permissions;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::package::Package;

use crate::download::download_and_unpack;
use crate::pkgscript::{Instruction, Parser};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use fs_extra::dir::{move_dir, CopyOptions};
use globset::Glob;
use temp_dir::TempDir;

pub struct InstallOpts {
    pub force: bool,
    pub publish: bool,
}

struct InstallCtx {
    sources_dir: PathBuf,
    packages_dir: PathBuf,
    bin_dir: PathBuf,
    tmp_dir: TempDir,
    output_dir: PathBuf,
}

impl InstallCtx {
    pub fn new(root_dir: PathBuf, tmp_dir: TempDir) -> Self {
        Self {
            packages_dir: root_dir.join("packages"),
            bin_dir: root_dir.join("bin"),
            sources_dir: tmp_dir.child("sources"),
            output_dir: tmp_dir.child("output"),
            tmp_dir,
        }
    }
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

pub struct Installer {
    pkg: Package,
}

impl Installer {
    pub fn new(pkg: Package) -> Self {
        Installer { pkg }
    }

    fn fetch_sources(&self, ctx: &InstallCtx) -> Result<()> {
        let sources = self
            .pkg
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

            let checksum = download_and_unpack(&source.url, &ctx.sources_dir)?;

            if source.checksum != checksum {
                return Err(anyhow!(
                    "checksum mismatch for source '{}' (expected: '{}', got: '{}')",
                    source.url,
                    source.checksum,
                    checksum
                ));
            }
        }

        Ok(())
    }

    fn eval_pkgscript(&self, ctx: &InstallCtx) -> Result<Vec<String>> {
        println!("{}", ">> evaluate pkgscript".blue());

        let out_bin_dir = ctx.output_dir.join("bin");

        fs::create_dir_all(&out_bin_dir)?;

        let script = Parser::parse(&self.pkg.install)?;
        let mut published = vec![];

        for instruction in script.body {
            println!("{}", instruction.to_string().white());

            match instruction {
                Instruction::Package { source, target } => {
                    let source = if let Some(idx) = source.find('*') {
                        let pattern =
                            format!("{}/{}", ctx.tmp_dir.path().to_str().unwrap(), source);
                        let glob = Glob::new(&pattern)?;
                        let prefix = ctx.tmp_dir.child(&source[..idx]);

                        find_file(prefix.parent().unwrap(), glob)?
                    } else {
                        ctx.tmp_dir.child(source)
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

        Ok(published)
    }

    fn package(&self, ctx: &InstallCtx, force: bool) -> Result<()> {
        println!("{}", ">> packaging".blue());

        let packages_dir = ctx.packages_dir.join(&self.pkg.name);

        if !packages_dir.exists() {
            fs::create_dir_all(&packages_dir)?;
        }

        let mut copy_opts = CopyOptions::new();

        copy_opts.overwrite = force;
        copy_opts.content_only = true;

        let pkg_dir = packages_dir.join(&self.pkg.version);

        move_dir(&ctx.output_dir, &pkg_dir, &copy_opts).context("move to destination failed")?;

        Ok(())
    }

    fn publish(&self, ctx: &InstallCtx, published: Vec<String>) -> Result<()> {
        if !ctx.bin_dir.exists() {
            fs::create_dir_all(&ctx.bin_dir)?;
        }

        let pkg_bin_dir = ctx
            .packages_dir
            .join(&self.pkg.name)
            .join(&self.pkg.version)
            .join("bin");

        for target in published {
            symlink(pkg_bin_dir.join(&target), ctx.bin_dir.join(target))?;
        }

        Ok(())
    }

    pub fn install(self, root_dir: PathBuf, opts: InstallOpts) -> Result<()> {
        let ctx = InstallCtx::new(root_dir, TempDir::new()?);

        set_env_vars();

        self.fetch_sources(&ctx)?;
        let published = self.eval_pkgscript(&ctx)?;
        self.package(&ctx, opts.force)?;

        if opts.publish {
            self.publish(&ctx, published)?;
        }

        Ok(())
    }
}