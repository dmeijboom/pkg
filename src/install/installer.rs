use std::fs;
use std::fs::Permissions;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context as _, Result};
use fs_extra::dir::{move_dir, CopyOptions};
use globset::Glob;
use temp_dir::TempDir;
use tokio::sync::mpsc::channel;

use crate::download::download_and_unpack;
use crate::install::channel::{Receiver, Sender};
use crate::install::{Event, MessageType};
use crate::package::Package;
use crate::pkgscript::{Instruction, Parser};

#[derive(Debug, PartialEq)]
pub enum Stage {
    FetchSources,
    EvalPkgscript,
    Package,
    Publish,
}

pub struct Opts<'o> {
    pub os: &'o str,
    pub arch: &'o str,
    pub force: bool,
    pub stage: Stage,
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

struct Dirs {
    sources: PathBuf,
    packages: PathBuf,
    bin: PathBuf,
    tmp: TempDir,
    output: PathBuf,
}

pub struct Installer<'i> {
    pkg: &'i Package,
    dirs: Dirs,
    tx: Sender,
}

impl<'i> Installer<'i> {
    pub fn new(pkg: &'i Package, root: PathBuf) -> Result<(Self, Receiver)> {
        let (tx, rx) = channel(10);
        let tmp = TempDir::new()?;

        Ok((
            Installer {
                pkg,
                tx,
                dirs: Dirs {
                    packages: root.join("packages"),
                    bin: root.join("bin"),
                    sources: tmp.child("sources"),
                    output: tmp.child("output"),
                    tmp,
                },
            },
            rx,
        ))
    }

    async fn fetch_sources(&self, os: &str, arch: &str) -> Result<()> {
        let sources = self
            .pkg
            .sources
            .get(os)
            .and_then(|targets| targets.get(arch))
            .ok_or_else(|| anyhow!("no sources found for target: {}.{}", os, arch))?;

        for source in sources {
            self.tx
                .send(Event::Message(
                    MessageType::Info,
                    format!("downloading {}", source.url),
                ))
                .await?;

            let checksum = download_and_unpack(&source.url, &self.dirs.sources).await?;

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

    async fn eval_pkgscript(&self) -> Result<Vec<String>> {
        let out_bin_dir = self.dirs.output.join("bin");

        fs::create_dir_all(&out_bin_dir)?;

        let script = Parser::parse(&self.pkg.install)?;
        let mut published = vec![];

        for instruction in script.body {
            self.tx
                .send(Event::Message(MessageType::Info, instruction.to_string()))
                .await?;

            match instruction {
                Instruction::Package { source, target } => {
                    let source = if let Some(idx) = source.find('*') {
                        let pattern =
                            format!("{}/{}", self.dirs.tmp.path().to_str().unwrap(), source);
                        let glob = Glob::new(&pattern)?;
                        let prefix = self.dirs.tmp.child(&source[..idx]);

                        find_file(prefix.parent().unwrap(), glob)?
                    } else {
                        self.dirs.tmp.child(source)
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

    async fn package(&self, force: bool) -> Result<()> {
        let packages_dir = self.dirs.packages.join(&self.pkg.name);

        if !packages_dir.exists() {
            fs::create_dir_all(&packages_dir)?;
        }

        let mut copy_opts = CopyOptions::new();

        copy_opts.overwrite = force;
        copy_opts.content_only = true;

        let pkg_dir = packages_dir.join(&self.pkg.version);

        move_dir(&self.dirs.output, &pkg_dir, &copy_opts).context("move to destination failed")?;

        Ok(())
    }

    async fn publish(&self, published: Vec<String>) -> Result<()> {
        if !self.dirs.bin.exists() {
            fs::create_dir_all(&self.dirs.bin)?;
        }

        let pkg_bin_dir = self
            .dirs
            .packages
            .join(&self.pkg.name)
            .join(&self.pkg.version)
            .join("bin");

        for target in published {
            symlink(pkg_bin_dir.join(&target), self.dirs.bin.join(target))?;
        }

        Ok(())
    }

    pub async fn install(self, opts: Opts<'_>) -> Result<()> {
        self.tx.send(Event::EnterStage(Stage::FetchSources)).await?;
        self.fetch_sources(opts.os, opts.arch).await?;
        self.tx.send(Event::ExitStage(Stage::FetchSources)).await?;

        if opts.stage != Stage::FetchSources {
            self.tx
                .send(Event::EnterStage(Stage::EvalPkgscript))
                .await?;
            let published = self.eval_pkgscript().await?;
            self.tx.send(Event::ExitStage(Stage::EvalPkgscript)).await?;

            if opts.stage != Stage::EvalPkgscript {
                self.tx.send(Event::EnterStage(Stage::Package)).await?;
                self.package(opts.force).await?;
                self.tx.send(Event::ExitStage(Stage::Package)).await?;

                if opts.stage != Stage::Package {
                    self.tx.send(Event::EnterStage(Stage::Publish)).await?;
                    self.publish(published).await?;
                    self.tx.send(Event::ExitStage(Stage::Publish)).await?;
                }
            }
        }

        Ok(())
    }
}
