#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, from_binary, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, TokenInfoResponse};
use cw_utils::must_pay;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, GetRegistrationResponse, InfoForCodeIdResponse, InstantiateMsg,
    ListRegistrationsResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{
    Config, PaymentInfo, Registration, CHAIN_ID_CODE_ID_TO_NAME, CONFIG,
    NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION, NAME_CHAIN_ID_TO_OWNER,
    NAME_CHAIN_ID_VERSION_TO_CODE_ID,
};

const CONTRACT_NAME: &str = "crates.io:cw-code-id-registry";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn assert_cw20(deps: Deps, cw20_addr: &Addr) -> Result<(), ContractError> {
    let _resp: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(cw20_addr, &cw20_base::msg::QueryMsg::TokenInfo {})
        .map_err(|_err| ContractError::InvalidCw20 {})?;
    Ok(())
}

fn validate_payment_info(deps: Deps, payment_info: &PaymentInfo) -> Result<(), ContractError> {
    match payment_info {
        PaymentInfo::None {} => {}
        PaymentInfo::Cw20Payment {
            token_address,
            payment_amount,
        } => {
            if payment_amount.is_zero() {
                return Err(ContractError::IncorrectPaymentAmount {});
            }

            // Validate it is a valid CW20 address
            let payment_token_address = deps.api.addr_validate(token_address)?;
            assert_cw20(deps, &payment_token_address)?;
        }
        PaymentInfo::NativePayment { payment_amount, .. } => {
            if payment_amount.is_zero() {
                return Err(ContractError::IncorrectPaymentAmount {});
            }
        }
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_payment_info(deps.as_ref(), &msg.payment_info)?;
    let validated_admin = deps.api.addr_validate(&msg.admin)?;
    let config = Config {
        admin: validated_admin,
        payment_info: msg.payment_info,
    };

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(wrapped) => execute_receive(deps, info, wrapped),
        ExecuteMsg::Register {
            name,
            version,
            chain_id,
            code_id,
            checksum,
        } => execute_register(deps, info, name, version, chain_id, code_id, checksum),
        ExecuteMsg::SetOwner {
            name,
            chain_id,
            owner,
        } => execute_set_owner(deps, info.sender, name, chain_id, owner),
        ExecuteMsg::Unregister {
            name,
            chain_id,
            code_id,
        } => execute_unregister(deps, info.sender, name, chain_id, code_id),
        ExecuteMsg::UpdateConfig {
            admin,
            payment_info,
        } => execute_update_config(deps, env, info, admin, payment_info),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapped: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    match config.payment_info {
        PaymentInfo::Cw20Payment {
            token_address,
            payment_amount,
        } => {
            if info.sender != token_address {
                return Err(ContractError::UnrecognizedCw20 {});
            }

            let sender = deps.api.addr_validate(&wrapped.sender)?;
            let amount = wrapped.amount;
            let msg: ReceiveMsg = from_binary(&wrapped.msg)?;

            if payment_amount != amount {
                return Err(ContractError::IncorrectPaymentAmount {});
            }

            match msg {
                ReceiveMsg::Register {
                    name,
                    version,
                    chain_id,
                    code_id,
                    checksum,
                } => register_code_id(
                    deps,
                    amount,
                    name,
                    chain_id,
                    Registration {
                        registered_by: sender,
                        version,
                        code_id,
                        checksum,
                    },
                ),
            }
        }
        _ => Err(ContractError::InvalidPayment {}),
    }
}

pub fn execute_register(
    deps: DepsMut,
    info: MessageInfo,
    name: String,
    version: String,
    chain_id: String,
    code_id: u64,
    checksum: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    match config.payment_info {
        PaymentInfo::Cw20Payment { .. } => Err(ContractError::InvalidPayment {}),
        PaymentInfo::None {} => register_code_id(
            deps,
            Uint128::zero(),
            name,
            chain_id,
            Registration {
                registered_by: info.sender,
                version,
                code_id,
                checksum,
            },
        ),
        PaymentInfo::NativePayment {
            token_denom,
            payment_amount,
        } => {
            let amount_paid = must_pay(&info, &token_denom)?;
            if payment_amount != amount_paid {
                return Err(ContractError::IncorrectPaymentAmount {});
            }

            register_code_id(
                deps,
                amount_paid,
                name,
                chain_id,
                Registration {
                    registered_by: info.sender,
                    version,
                    code_id,
                    checksum,
                },
            )
        }
    }
}

pub fn execute_set_owner(
    deps: DepsMut,
    sender: Addr,
    name: String,
    chain_id: String,
    owner: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only allow admin or existing owner to set the owner.
    let existing_owner =
        NAME_CHAIN_ID_TO_OWNER.may_load(deps.storage, (name.clone(), chain_id.clone()))?;
    if sender != config.admin && Some(sender) != existing_owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner.as_ref() {
        let new_owner = deps.api.addr_validate(owner)?;
        // Update owner.
        NAME_CHAIN_ID_TO_OWNER.save(deps.storage, (name.clone(), chain_id.clone()), &new_owner)?;
    } else {
        // Clear owner.
        NAME_CHAIN_ID_TO_OWNER.remove(deps.storage, (name.clone(), chain_id.clone()));
    }

    Ok(Response::new()
        .add_attribute("action", "set_owner")
        .add_attribute("name", name)
        .add_attribute("chain_id", chain_id)
        .add_attribute("owner", owner.unwrap_or_default()))
}

pub fn execute_unregister(
    deps: DepsMut,
    sender: Addr,
    name: String,
    chain_id: String,
    code_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Only allow admin to unregister.
    if sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Retrieve registration.
    let registration = NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION
        .load(deps.storage, (name.clone(), chain_id.clone(), code_id))
        .map_err(|_| ContractError::NotFound {})?;

    // Remove from state.
    CHAIN_ID_CODE_ID_TO_NAME.remove(deps.storage, (chain_id.clone(), code_id));
    NAME_CHAIN_ID_VERSION_TO_CODE_ID.remove(
        deps.storage,
        (name.clone(), chain_id.clone(), registration.version),
    );
    NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION
        .remove(deps.storage, (name.clone(), chain_id.clone(), code_id));

    Ok(Response::new()
        .add_attribute("action", "unregister")
        .add_attribute("chain_id", chain_id)
        .add_attribute("name", name)
        .add_attribute("code_id", code_id.to_string()))
}

pub fn register_code_id(
    deps: DepsMut,
    amount_sent: Uint128,
    name: String,
    chain_id: String,
    registration: Registration,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let existing_owner =
        NAME_CHAIN_ID_TO_OWNER.may_load(deps.storage, (name.clone(), chain_id.clone()))?;

    // If not admin, ensure sender has access to register.
    if registration.registered_by != config.admin {
        // If has owner set, ensure sender is owner.
        if let Some(owner) = existing_owner {
            if registration.registered_by != owner {
                return Err(ContractError::Unauthorized {});
            }
            // If no owner set, unauthorized.
        } else {
            return Err(ContractError::Unauthorized {});
        }
    }

    // Can't re-register a code ID on a chain.
    if CHAIN_ID_CODE_ID_TO_NAME
        .may_load(deps.storage, (chain_id.clone(), registration.code_id))?
        .is_some()
    {
        return Err(ContractError::CodeIDAlreadyRegistered(
            registration.code_id,
            chain_id,
        ));
    }

    // Can't re-register a version.
    if NAME_CHAIN_ID_VERSION_TO_CODE_ID
        .may_load(
            deps.storage,
            (name.clone(), chain_id.clone(), registration.version.clone()),
        )?
        .is_some()
    {
        return Err(ContractError::VersionAlreadyRegistered(
            registration.version,
            name,
            chain_id,
        ));
    };

    // Add to state.
    NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION.save(
        deps.storage,
        (name.clone(), chain_id.clone(), registration.code_id),
        &registration,
    )?;
    NAME_CHAIN_ID_VERSION_TO_CODE_ID.save(
        deps.storage,
        (name.clone(), chain_id.clone(), registration.version.clone()),
        &registration.code_id,
    )?;
    CHAIN_ID_CODE_ID_TO_NAME.save(deps.storage, (chain_id, registration.code_id), &name)?;

    // Send payment to admin.
    let msgs = if amount_sent > Uint128::zero() {
        match config.payment_info {
            PaymentInfo::None {} => vec![],
            PaymentInfo::NativePayment { token_denom, .. } => {
                vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: config.admin.to_string(),
                    amount: coins(amount_sent.u128(), token_denom),
                })]
            }
            PaymentInfo::Cw20Payment { token_address, .. } => {
                vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: token_address,
                    msg: to_binary(&cw20_base::msg::ExecuteMsg::Transfer {
                        recipient: config.admin.to_string(),
                        amount: amount_sent,
                    })?,
                    funds: vec![],
                })]
            }
        }
    } else {
        vec![]
    };

    Ok(Response::new()
        .add_attribute("action", "register")
        .add_messages(msgs))
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_admin: Option<String>,
    new_payment_info: Option<PaymentInfo>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let new_payment_info = new_payment_info.unwrap_or_else(|| config.clone().payment_info);
    let new_admin = new_admin.unwrap_or_else(|| config.admin.to_string());

    // Validate admin address
    let admin = deps.api.addr_validate(&new_admin)?;

    // Validate payment info
    validate_payment_info(deps.as_ref(), &new_payment_info)?;

    config.admin = admin;
    config.payment_info = new_payment_info;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::GetRegistration {
            name,
            chain_id,
            version,
        } => query_get_registration(deps, name, chain_id, version),
        QueryMsg::InfoForCodeId { chain_id, code_id } => {
            query_info_for_code_id(deps, chain_id, code_id)
        }
        QueryMsg::ListRegistrations { name, chain_id } => {
            query_list_registrations(deps, name, chain_id)
        }
    }
}

pub fn query_get_registration(
    deps: Deps,
    name: String,
    chain_id: String,
    version: Option<String>,
) -> StdResult<Binary> {
    let registration = if let Some(version) = version {
        // Get specific version if passed.
        let code_id = NAME_CHAIN_ID_VERSION_TO_CODE_ID
            .load(deps.storage, (name.clone(), chain_id.clone(), version))
            .map_err(|_| StdError::GenericErr {
                msg: ContractError::NotFound {}.to_string(),
            })?;

        NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION
            .load(deps.storage, (name, chain_id, code_id))
            .map_err(|_| StdError::GenericErr {
                msg: ContractError::NotFound {}.to_string(),
            })?
    } else {
        // Get most recent code ID registration.
        NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION
            .prefix((name, chain_id))
            .range(deps.storage, None, None, Order::Descending)
            .next()
            .ok_or(StdError::GenericErr {
                msg: ContractError::NotFound {}.to_string(),
            })?
            .map_err(|_| StdError::GenericErr {
                msg: ContractError::NotFound {}.to_string(),
            })?
            .1
    };
    to_binary(&GetRegistrationResponse { registration })
}

pub fn query_info_for_code_id(deps: Deps, chain_id: String, code_id: u64) -> StdResult<Binary> {
    // Retrieve contract name.
    let name = CHAIN_ID_CODE_ID_TO_NAME
        .load(deps.storage, (chain_id.clone(), code_id))
        .map_err(|_| StdError::GenericErr {
            msg: ContractError::NotFound {}.to_string(),
        })?;

    // Retrieve registration.
    let registration = NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION
        .load(deps.storage, (name.clone(), chain_id, code_id))
        .map_err(|_| StdError::GenericErr {
            msg: ContractError::NotFound {}.to_string(),
        })?;

    to_binary(&InfoForCodeIdResponse {
        registered_by: registration.registered_by,
        name,
        version: registration.version,
        checksum: registration.checksum,
    })
}

// TODO: Paginate.
pub fn query_list_registrations(deps: Deps, name: String, chain_id: String) -> StdResult<Binary> {
    let registrations = NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION
        .prefix((name, chain_id))
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(u64, Registration)>>>()?
        .into_iter()
        .map(|(_, registration)| registration)
        .collect();
    to_binary(&ListRegistrationsResponse { registrations })
}
