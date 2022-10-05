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
    /// Download the code ids into `cpm.lock` for each configured contract dependency
    Install,
    /// Upgrade to the latest versions for each contract dependency in `cpm.yaml`
    Upgrade,
    /// Bump version, store on chain, and release each contract package in `cpm.yaml`
    Release {
        /// Cosmos chain registry chain_id to use for contract storage
        #[clap(long, action)]
        chain_reg_id: String,

        /// Path to Cargo.toml
        #[clap(long, action)]
        cargo_path: PathBuf,

        /// OS Keyring key_name for corresponding mnemonic to use for wasm storage
        #[clap(short, long, action)]
        store_key_name: String,

        /// OS Keyring key_name for corresponding mnemonic to use for DAO proposal
        #[clap(short, long, action)]
        prop_key_name: String,

        /// New version to release the packages under
        #[clap(value_parser)]
        version: String,
    },
}
