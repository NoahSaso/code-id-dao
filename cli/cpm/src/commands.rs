use anyhow::{Context, Result};
use cosm_orc::config::cfg::{ChainConfig, Config, ConfigInput};
use cosm_orc::config::key::{Key, KeyringParams, SigningKey};
use cosm_orc::orchestrator::cosm_orc::CosmOrc;
use cosmwasm_std::{to_binary, CosmosMsg, WasmMsg};
use cw_code_id_registry::msg::{GetRegistrationResponse, QueryMsg};
use cw_core::query::DumpStateResponse;
use cw_core_interface::voting::InfoResponse;
use cw_proposal_multiple::state::{MultipleChoiceOption, MultipleChoiceOptions};
use std::collections::HashMap;
use std::env::consts::ARCH;
use std::fs::{self, File};
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;
use voting::deposit::CheckedDepositInfo;

use crate::{CPMConfig, LockDep, LockFile};

fn init_orc(chain_id: String) -> Result<(CosmOrc, String)> {
    let cfg_input = ConfigInput {
        chain_cfg: ChainConfig::ChainRegistry(chain_id.clone()),
        contract_deploy_info: HashMap::new(),
    };
    let cfg = Config::from_config_input(cfg_input)?;
    Ok((CosmOrc::new(cfg.clone(), false)?, cfg.chain_cfg.chain_id))
}

pub(crate) fn install(cfg: &CPMConfig, config_dir: &str) -> Result<()> {
    if cfg.dependencies.is_empty() {
        return Err(anyhow::anyhow!(
            "cpm.yaml must have at least one dependency to install"
        ));
    }

    let (mut orc, _) = init_orc(cfg.chain_id.clone())?;

    let mut lock_file = LockFile {
        dependencies: vec![],
    };

    for (registry_addr, contracts) in &cfg.dependencies {
        orc.contract_map.add_address("registry", registry_addr)?;

        for (chain_id, deps) in contracts {
            for (contract_name, dep) in deps {
                let res: GetRegistrationResponse = orc
                    .query(
                        "registry",
                        &QueryMsg::GetRegistration {
                            name: contract_name.clone(),
                            chain_id: chain_id.clone(),
                            version: dep.version.clone(),
                        },
                    )
                    .context(format!(
                        "contract not found in chain registry {contract_name} @ {chain_id}"
                    ))?
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
    }

    // TODO: Sort the dependencies in alpha order to make the lockfile deterministic

    fs::write(
        format!("{config_dir}/cpm.lock"),
        serde_yaml::to_string(&lock_file)?,
    )?;

    println!("Finished! \nCheck cpm.lock file for code_ids");

    Ok(())
}

// NOTE: You must be a member of the configured `Release.dao_addr` to release contract versions.
//
// `cpm release` will build and optimize the smart contracts in the provided `Cargo.toml`
// and then store the optimized wasms on chain and create a Dao Dao proposal to release new versions.

// TODO: When cargo building wasms the .cargo directory with the username is baked in the resulting binary (and accessible via `strings`).
// This can dox people. I should see if I can fix this here or in cw-optimizoor.
pub(crate) fn release(
    cfg: &CPMConfig,
    config_dir: &str,
    cargo_path: &PathBuf,
    store_key_name: &str,
    prop_key_name: &str,
    chain_reg_id: &str,
    version: String,
) -> Result<()> {
    let cargo_path = cargo_path.to_str().context("invalid cargo_path")?;
    let release = cfg
        .release
        .as_ref()
        .context("missing release section in cpm.yaml")?;

    if release.packages.len() == 0 {
        return Err(anyhow::anyhow!(
            "cpm.yaml must have at least one package to release"
        ));
    }

    bump_version(version.clone(), cfg.clone(), config_dir)?;

    let (mut store_orc, storage_chain_id) = init_orc(chain_reg_id.to_string())?;

    optimize_contracts(&store_orc, cargo_path)?;

    // TODO: Call `store_contract()` instead and call it only for the packages configured in `cpm.yaml`
    let artifacts_dir = format!("{config_dir}/artifacts");
    let responses = store_orc.store_contracts(
        &artifacts_dir,
        &signing_key(store_key_name.to_string()),
        None,
    )?;

    let mut store_hashes = HashMap::new();
    for res in responses {
        store_hashes.insert(res.code_id, res.tx_hash);
    }

    let mut checksum_map = HashMap::new();
    let checksum = format!("{artifacts_dir}/checksums.txt");
    let reader = BufReader::new(File::open(checksum)?);

    for line in reader.lines() {
        let parts: Vec<String> = line?.split("  ").map(|s| s.to_string()).collect();
        let (sha, mut contract) = (parts[0].clone(), parts[1].clone());

        // remove wasm from contract name:
        let arch_suffix = format!("-{}.wasm", ARCH);
        if contract.to_string().ends_with(&arch_suffix) {
            contract = contract.trim_end_matches(&arch_suffix).to_string();
        } else {
            contract = contract.trim_end_matches(&".wasm").to_string();
        }
        checksum_map.insert(contract, sha);
    }

    // make a proposal in the configured DAO to register new versions:

    let (mut dao_orc, _) = init_orc(cfg.chain_id.clone())?;
    dao_orc.contract_map.add_address("dao", &release.dao_addr)?;

    let dao: DumpStateResponse = dao_orc
        .query("dao", &cw_core::msg::QueryMsg::DumpState {})?
        .data()?;

    let prop_addr = &dao.proposal_modules[0].address;
    dao_orc.contract_map.add_address("dao_prop", prop_addr)?;

    let mut register_msgs = vec![];
    for pkg in &release.packages {
        let msg = &cw_code_id_registry::msg::ExecuteMsg::Register {
            name: pkg.name.clone(),
            version: version.clone(),
            chain_id: storage_chain_id.clone(),
            code_id: store_orc.contract_map.code_id(&pkg.name).context(format!(
                "package name ({}) different from contract name",
                pkg.name
            ))?,
            checksum: checksum_map
                .get(&format!("{}", pkg.name))
                .context("invalid package name")?
                .to_string(),
        };

        println!(
            "Preparing {}:{} @ {} - \n{:?}\n",
            pkg.name,
            version,
            storage_chain_id.clone(),
            msg
        );

        register_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: release.dao_addr.clone(),
            msg: to_binary(&msg)?,
            funds: vec![],
        }));
    }

    // TODO: Print gas estimate for the proposal
    println!(
        "Release ({}) contract @ {}? [y/N]",
        register_msgs.len(),
        &version
    );

    let mut input = String::new();
    if let Ok(_) = io::stdin().read_line(&mut input) {
        if input.trim() == "y" {
            println!("creating register version proposal!");

            // TODO: If there are more than one package being released, make that clear
            // Get the workspace name??
            let package_name = release.packages[0].name.clone();
            let key = signing_key(prop_key_name.to_string());

            create_proposal(
                &version,
                chain_reg_id,
                &package_name,
                store_hashes,
                register_msgs,
                &mut dao_orc,
                dao.voting_module.into(),
                prop_addr.to_string(),
                &key,
            )?;
            return Ok(());
        }
    }

    println!("skipping. . .");

    Ok(())
}

// bump smart contract version to `version` in cpm.yaml:
fn bump_version(version: String, mut cfg: CPMConfig, config_dir: &str) -> Result<()> {
    for pkg in &mut cfg.release.as_mut().unwrap().packages {
        pkg.version = version.clone()
    }
    fs::write(
        format!("{config_dir}/cpm.yaml"),
        serde_yaml::to_string(&cfg)?,
    )?;

    Ok(())
}

#[allow(unused_variables)]
fn optimize_contracts(orc: &CosmOrc, cargo_path: &str) -> Result<()> {
    #[cfg(feature = "optimize")]
    orc.optimize_contracts(cargo_path)?;

    Ok(())
}

fn signing_key(key_name: String) -> SigningKey {
    SigningKey {
        name: "cpm".to_string(),
        key: Key::Keyring(KeyringParams {
            service: "cpm".to_string(),
            key_name,
        }),
    }
}

fn create_proposal(
    version: &str,
    chain_reg_id: &str,
    package_name: &str,
    store_hashes: HashMap<u64, String>,
    register_msgs: Vec<CosmosMsg<cosmwasm_std::Empty>>,
    orc: &mut CosmOrc,
    voting_module: String,
    prop_addr: String,
    key: &SigningKey,
) -> Result<()> {
    let res: InfoResponse = orc
        .query("dao_prop", &cw_proposal_single::msg::QueryMsg::Info {})?
        .data()?;

    let is_single_prop = res.info.contract.contains("single");

    if let Some(deposit) = proposal_deposit(is_single_prop, &orc)? {
        // increase prop module allowance to pay for deposit
        orc.contract_map.add_address("dao_vote", voting_module)?;

        let res = orc.query(
            "dao_vote",
            // TODO: Support all voting modules, dont just assume its `cw20_staked_balance_voting`
            &cw20_staked_balance_voting::msg::QueryMsg::TokenContract {},
        )?;
        let token_addr: &str = res.data()?;

        orc.contract_map.add_address("dao_tok", token_addr)?;
        orc.execute(
            "dao_tok",
            "dao_prop_e",
            &cw20_base::msg::ExecuteMsg::IncreaseAllowance {
                spender: prop_addr,
                amount: deposit.deposit,
                expires: None,
            },
            &key,
            vec![],
        )?;
    }

    let title = format!("Release {package_name}:{version} - {chain_reg_id}");
    let mut mintscan_urls = String::new();
    for (code_id, _) in store_hashes {
        mintscan_urls += &format!(
            "  - [Mintscan wasm link: {code_id}](https://www.mintscan.io/{chain_reg_id}/wasm/code/{code_id}) \n"
        );
    }
    let description = format!("{mintscan_urls} \n \n ~Generated by cpm cli~");

    if is_single_prop {
        orc.execute(
            "dao_prop",
            "dao_prop_e",
            &cw_proposal_single::msg::ExecuteMsg::Propose {
                title,
                description,
                msgs: register_msgs,
            },
            &key,
            vec![],
        )?;
    } else {
        orc.execute(
            "dao_prop",
            "dao_prop_e",
            &cw_proposal_multiple::msg::ExecuteMsg::Propose {
                title,
                description,
                choices: MultipleChoiceOptions {
                    options: vec![MultipleChoiceOption {
                        description: "Register new contract versions".to_string(),
                        msgs: Some(register_msgs),
                    }],
                },
            },
            &key,
            vec![],
        )?;
    }

    Ok(())
}

fn proposal_deposit(is_single_choice: bool, orc: &CosmOrc) -> Result<Option<CheckedDepositInfo>> {
    let deposit = if is_single_choice {
        let cfg: cw_proposal_single::state::Config = orc
            .query("dao_prop", &cw_proposal_single::msg::QueryMsg::Config {})?
            .data()?;
        cfg.deposit_info
    } else {
        let cfg: cw_proposal_multiple::state::Config = orc
            .query("dao_prop", &cw_proposal_multiple::msg::QueryMsg::Config {})?
            .data()?;
        cfg.deposit_info
    };

    Ok(deposit)
}
