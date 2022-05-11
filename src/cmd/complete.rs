use std::io;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};

use crate::Args;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(long, arg_enum)]
    generator: Shell,
}

pub fn run(opts: Opts) -> Result<()> {
    let mut cmd = Args::command();

    let name = cmd.get_name().to_string();

    generate(opts.generator, &mut cmd, name, &mut io::stdout());

    Ok(())
}
