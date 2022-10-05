use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use cosmwasm_std::Addr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};

mod cli;
mod commands;

// TODO: Add verbose/debug flag

pub type RegistryAddr = String;
pub type ContractName = String;
pub type ChainID = String;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CPMConfig {
    /// populates chain info from the [chain registry](https://github.com/cosmos/chain-registry)
    pub chain_id: String,
    /// optional contract packages to release
    pub release: Option<Release>,
    /// contract dependencies to download code_ids for into lock file
    pub dependencies: HashMap<RegistryAddr, HashMap<ChainID, HashMap<ContractName, Dependency>>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Release {
    /// DAO that is admin over the package's cw_code_id_registry instance.
    /// Will be used to create release proposals.
    pub dao_addr: String,
    /// contract packages to release
    pub packages: Vec<Package>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Package {
    name: ContractName,
    version: String,
    description: String,
    // TODO: Make these optional:
    authors: Vec<String>,
    keywords: Vec<String>,
    website: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Dependency {
    /// Optional version, if ommitted latest version of contract will be used
    pub version: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LockFile {
    pub dependencies: Vec<LockDep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LockDep {
    pub name: ContractName,
    pub chain_id: ChainID,
    pub registered_by: Addr,
    pub version: String,
    pub code_id: u64,
    pub checksum: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let config_dir = config_dir(cli.config)?;

    let file = &fs::read(format!("{config_dir}/cpm.yaml")).context(format!(
        "cpm.yaml file not found in: {config_dir}. See --help"
    ))?;

    let cfg: CPMConfig = serde_yaml::from_slice(file)?;

    // process cli subcommand:
    if let Some(cmd) = &cli.command {
        match cmd {
            Commands::Init => todo!(),
            Commands::Install => commands::install(&cfg, &config_dir),
            Commands::Upgrade => todo!(),
            Commands::Release {
                cargo_path,
                store_key_name,
                prop_key_name,
                chain_reg_id,
                version,
            } => commands::release(
                &cfg,
                &config_dir,
                cargo_path,
                store_key_name,
                prop_key_name,
                chain_reg_id,
                version.to_string(),
            ),
        }?;
    }

    Ok(())
}

/// get directory containing the `cpm.yaml` config file:
fn config_dir(cfg: Option<PathBuf>) -> Result<String> {
    let config_dir: PathBuf = if let Some(config) = cfg {
        if config.is_file() {
            config.parent().context("invalid config dir")?.to_path_buf()
        } else {
            config
        }
    } else {
        env::current_dir()?
    };

    let mut config_dir = config_dir.to_str().context("invalid dir name")?;
    if config_dir == "" {
        config_dir = ".";
    }

    Ok(config_dir.to_string())
}
