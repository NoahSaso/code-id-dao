use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Location of cpm.yaml config file or containing directory.
    /// Defaults to current directory.
    #[clap(short, long, value_parser, value_name = "FILE")]
    pub config: Option<PathBuf>,

    #[clap(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initializes a new cpm.yaml config
    Init,
    /// Download the code ids into `cpm.lock` for each configured contract version
    Install,
    /// Upgrade to the latest versions for each contract stored in `cpm.yaml`
    Upgrade,
    /// Release a new version of your smart contract package stored in `cpm.yaml`
    Release,
}
