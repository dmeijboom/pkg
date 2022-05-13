use serde::Deserialize;
use serde_dhall::StaticType;
use strum_macros::AsRefStr;

#[allow(non_camel_case_types)]
#[allow(clippy::enum_variant_names)]
#[derive(Deserialize, Debug, PartialEq, AsRefStr, StaticType)]
pub enum OS {
    Unknown,
    Linux,
    MacOS,
    iOS,
    FreeBSD,
    DragonFly,
    NetBSD,
    OpenBSD,
    Solaris,
    Android,
    Windows,
}

impl Default for OS {
    fn default() -> Self {
        OS::Unknown
    }
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, PartialEq, AsRefStr, StaticType)]
pub enum Arch {
    Unknown,
    x86,
    x86_64,
    arm,
    arm64,
    aarch64,
    m68k,
    mips,
    mips64,
    powerpc,
    powerpc64,
    riscv64,
    s390x,
    sparc64,
}

impl Default for Arch {
    fn default() -> Self {
        Arch::Unknown
    }
}

#[derive(Deserialize, Debug, StaticType)]
pub struct Source {
    #[serde(default)]
    pub os: OS,
    #[serde(default)]
    pub arch: Arch,
    pub url: String,
}

#[derive(Deserialize, Debug, StaticType)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub sources: Vec<Source>,
    pub install: String,
}
