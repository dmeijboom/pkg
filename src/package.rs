use serde::{Deserialize, Serialize};
use serde_dhall::StaticType;

#[derive(Debug, Serialize, Deserialize, StaticType)]
pub struct Source {
    pub url: String,
    pub checksum: String,
}

macro_rules! impl_target {
    ($($name:ident),+) => {
        #[allow(non_camel_case_types)]
        #[derive(Serialize, Deserialize, Default, Debug, StaticType)]
        pub struct Targets {
            $(#[serde(default)]
            pub $name: Vec<Source>,)+
        }

        impl Targets {
            pub fn get(&self, name: &str) -> Option<&[Source]> {
                match name {
                    $(stringify!($name) if self.$name.is_empty() => None,
                    stringify!($name) => Some(&self.$name),)+
                    _ => None,
                }
            }

            pub fn valid_keys(&self) -> Vec<&str> {
                let mut keys = vec![];

                $(if !self.$name.is_empty() {
                    keys.push(stringify!($name));
                })+

                keys
            }

            pub fn is_empty(&self) -> bool {
                self.valid_keys().is_empty()
            }
        }
    };
}

impl_target!(
    unknown, x86, x86_64, arm, aarch64, m68k, mips, mips64, powerpc, powerpc64, riscv64, s390x,
    sparc64
);

macro_rules! impl_sources {
    ($($name:ident),+) => {
        #[derive(Serialize, Deserialize, Default, Debug, StaticType)]
        pub struct Sources {
            $(#[serde(default)]
            pub $name: Targets,)+
        }

        impl Sources {
            pub fn get(&self, name: &str) -> Option<&Targets> {
                match name {
                    $(stringify!($name) if self.$name.is_empty() => None,
                    stringify!($name) => Some(&self.$name),)+
                    _ => None,
                }
            }

            pub fn keys(&self) -> &[&str] {
                &[$(stringify!($name),)+]
            }
        }
    };
}

impl_sources!(
    unknown, linux, macos, ios, freebsd, dragonfly, netbsd, openbsd, solaris, android, windows
);

#[derive(Serialize, Deserialize, Debug, StaticType)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub sources: Sources,
    pub install: String,
}
