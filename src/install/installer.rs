use std::collections::HashMap;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use globset::Glob;
use temp_dir::TempDir;
use tokio::fs;
use tokio::fs::symlink;
use tokio::sync::mpsc::channel;

use crate::download::download_and_unpack;
use crate::install::channel::{Receiver, Sender};
use crate::install::{Event, MessageType};
use crate::package::Package;
use crate::pkgscript::{Instruction, Parser};
use crate::store::{Content, ContentType};
use crate::utils::sha256sum;

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

pub struct InstallResult {
    pub content: Vec<Content>,
}

impl InstallResult {
    pub fn new(content: Vec<Content>) -> Self {
        Self { content }
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

struct Dirs {
    sources: PathBuf,
    content: PathBuf,
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
                    content: root.join("content"),
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

    async fn eval_pkgscript(&self) -> Result<HashMap<PathBuf, Content>> {
        let out_bin_dir = self.dirs.output.join("bin");

        fs::create_dir_all(&out_bin_dir).await?;

        let script = Parser::parse(&self.pkg.install)?;
        let mut content_map = HashMap::new();

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
                    let filename = match target {
                        Some(target) => target,
                        None => source
                            .file_name()
                            .and_then(|f| f.to_str())
                            .unwrap()
                            .to_string(),
                    };

                    if !source.exists() {
                        return Err(anyhow!("source does not exist"));
                    }

                    let body = fs::read(&source).await?;
                    let checksum = sha256sum(body);

                    content_map.insert(
                        source,
                        Content::new(ContentType::Executable, filename, checksum),
                    );
                }
                Instruction::Publish { target } => {
                    if PathBuf::from(&target).components().count() > 1 {
                        return Err(anyhow!("publish target must contain only the filename"));
                    }

                    if let Some((_, content)) = content_map
                        .iter_mut()
                        .filter(|(_, c)| c.filename == target)
                        .next()
                    {
                        content.published = true;
                        continue;
                    }

                    return Err(anyhow!("unable to publish unpackaged target: {}", target));
                }
            }
        }

        Ok(content_map)
    }

    async fn package(&self, content_map: &HashMap<PathBuf, Content>) -> Result<()> {
        if !self.dirs.content.exists() {
            fs::create_dir_all(&self.dirs.content).await?;
        }

        for (source, content) in content_map {
            let dest = self.dirs.content.join(&content.checksum);

            fs::copy(source, &dest).await?;

            if content.content_type == ContentType::Executable {
                fs::set_permissions(dest, Permissions::from_mode(0o755)).await?;
            }
        }

        Ok(())
    }

    async fn publish(&self, content_map: &HashMap<PathBuf, Content>) -> Result<()> {
        if !self.dirs.bin.exists() {
            fs::create_dir_all(&self.dirs.bin).await?;
        }

        for (_, content) in content_map {
            let link = self.dirs.bin.join(&content.filename);

            if link.exists() {
                fs::remove_file(&link).await?;
            }

            symlink(self.dirs.content.join(&content.checksum), link).await?;
        }

        Ok(())
    }

    pub async fn install(self, opts: Opts<'_>) -> Result<InstallResult> {
        self.tx.send(Event::EnterStage(Stage::FetchSources)).await?;
        self.fetch_sources(opts.os, opts.arch).await?;
        self.tx.send(Event::ExitStage(Stage::FetchSources)).await?;

        if opts.stage != Stage::FetchSources {
            self.tx
                .send(Event::EnterStage(Stage::EvalPkgscript))
                .await?;
            let content_map = self.eval_pkgscript().await?;
            self.tx.send(Event::ExitStage(Stage::EvalPkgscript)).await?;

            if opts.stage != Stage::EvalPkgscript {
                self.tx.send(Event::EnterStage(Stage::Package)).await?;
                self.package(&content_map).await?;
                self.tx.send(Event::ExitStage(Stage::Package)).await?;

                if opts.stage != Stage::Package {
                    self.tx.send(Event::EnterStage(Stage::Publish)).await?;
                    self.publish(&content_map).await?;
                    self.tx.send(Event::ExitStage(Stage::Publish)).await?;

                    return Ok(InstallResult::new(
                        content_map.into_values().collect::<Vec<_>>(),
                    ));
                }
            }
        }

        Ok(InstallResult::new(vec![]))
    }
}
