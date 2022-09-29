use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use cosm_orc::config::cfg::{ChainConfig, Config, ConfigInput};
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosm_orc::orchestrator::deploy::DeployInfo;
use cosmwasm_std::Addr;
use cw_code_id_registry::msg::{GetRegistrationResponse, QueryMsg};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};

mod cli;

// TODO: Add verbose/debug flag
const CPM_REGISTRY_NAME: &str = "cosmwasm-package-manager";
pub type ContractName = String;
pub type ChainID = String;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CPMConfig {
    /// registry_addr is the contract address to use as the source of truth for all code id registration
    pub registry_addr: String,
    /// chain_id where cw-code-id-registry contract is instantiated.
    /// populates chain info from the [chain registry](https://github.com/cosmos/chain-registry)
    pub chain_id: String,
    /// contract dependencies to download code_ids for into lock file
    pub dependencies: HashMap<ChainID, HashMap<ContractName, Dependency>>,
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

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let config_dir = config_dir(cli.config)?;

    let file = &fs::read(format!("{}/cpm.yaml", &config_dir)).context(format!(
        "cpm.yaml file not found in: {}. See --help",
        config_dir
    ))?;

    let cfg: CPMConfig = serde_yaml::from_slice(file)?;

    let cfg_input = ConfigInput {
        chain_cfg: ChainConfig::ChainRegistry(cfg.chain_id.clone()),
        contract_deploy_info: HashMap::from([(
            CPM_REGISTRY_NAME.to_string(),
            DeployInfo {
                address: Some(cfg.registry_addr.clone()),
                // NOTE: We dont use the code_id because its already deployed
                code_id: 0,
            },
        )]),
    };
    let orc = CosmOrc::new(Config::from_config_input(cfg_input)?, false)?;

    // process cli subcommand:
    if let Some(cmd) = &cli.command {
        match cmd {
            Commands::Init => todo!(),
            Commands::Install => install(&cfg, &config_dir, &orc),
            Commands::Upgrade => todo!(),
            Commands::Release => todo!(),
        }?;
    }

    Ok(())
}

fn install(cfg: &CPMConfig, config_dir: &str, orc: &CosmOrc) -> Result<()> {
    let mut lock_file = LockFile {
        dependencies: vec![],
    };

    for (chain_id, deps) in &cfg.dependencies {
        for (contract_name, dep) in deps {
            let res: GetRegistrationResponse = orc
                .query(
                    CPM_REGISTRY_NAME,
                    &QueryMsg::GetRegistration {
                        name: contract_name.clone(),
                        chain_id: chain_id.clone(),
                        version: dep.version.clone(),
                    },
                )?
                .data()?;

            let reg = res.registration;
            lock_file.dependencies.push(LockDep {
                name: contract_name.clone(),
                chain_id: chain_id.clone(),
                registered_by: reg.registered_by,
                version: reg.version,
                code_id: reg.code_id,
                checksum: reg.checksum,
            })
        }
    }

    // TODO: Sort the dependencies in alpha order to make the lockfile deterministic

    fs::write(
        format!("{config_dir}/cpm.lock"),
        serde_yaml::to_string(&lock_file)?,
    )?;

    println!("Finished! \nCheck cpm.lock file for code_ids");

    Ok(())
}
