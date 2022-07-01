use crate::msg::{
    ExecuteMsg, GetRegistrationResponse, InfoForCodeIdResponse, InstantiateMsg,
    ListRegistrationsResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, PaymentInfo, Registration};
use crate::ContractError;
use anyhow::Result as AnyResult;
use cosmwasm_std::{coins, to_binary, Addr, Coin, Empty, Uint128};
use cw20::{BalanceResponse, Cw20Coin};
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

const USER_ADDR: &str = "user";
const OTHER_USER_ADDR: &str = "other_user";
const ADMIN_ADDR: &str = "admin";
const CHAIN_ID: &str = "chain-id";

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn registry_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn setup_app() -> App {
    let amount = Uint128::new(10000);
    App::new(|r, _a, s| {
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(USER_ADDR),
                vec![
                    Coin {
                        denom: "ujuno".to_string(),
                        amount,
                    },
                    Coin {
                        denom: "uatom".to_string(),
                        amount,
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(OTHER_USER_ADDR),
                vec![
                    Coin {
                        denom: "ujuno".to_string(),
                        amount,
                    },
                    Coin {
                        denom: "uatom".to_string(),
                        amount,
                    },
                ],
            )
            .unwrap();
        r.bank
            .init_balance(
                s,
                &Addr::unchecked(ADMIN_ADDR),
                vec![
                    Coin {
                        denom: "ujuno".to_string(),
                        amount,
                    },
                    Coin {
                        denom: "uatom".to_string(),
                        amount,
                    },
                ],
            )
            .unwrap();
    })
}

fn create_token(app: &mut App) -> Addr {
    let cw20_id = app.store_code(cw20_contract());
    app.instantiate_contract(
        cw20_id,
        Addr::unchecked(ADMIN_ADDR),
        &cw20_base::msg::InstantiateMsg {
            name: "Name Registry Token".to_string(),
            symbol: "NAME".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: USER_ADDR.to_string(),
                    amount: Uint128::new(1000),
                },
                Cw20Coin {
                    address: ADMIN_ADDR.to_string(),
                    amount: Uint128::new(1000),
                },
                Cw20Coin {
                    address: OTHER_USER_ADDR.to_string(),
                    amount: Uint128::new(1000),
                },
            ],
            mint: None,
            marketing: None,
        },
        &[],
        "some token",
        None,
    )
    .unwrap()
}

fn setup_test_case(app: &mut App, payment_info: PaymentInfo) -> Addr {
    let code_id = app.store_code(registry_contract());
    app.instantiate_contract(
        code_id,
        Addr::unchecked(ADMIN_ADDR),
        &InstantiateMsg {
            admin: ADMIN_ADDR.to_string(),
            payment_info,
        },
        &[],
        "Code ID Registry",
        None,
    )
    .unwrap()
}

#[test]
fn test_instantiate() {
    let mut app = setup_app();
    let token_addr = create_token(&mut app);
    let code_id = app.store_code(registry_contract());

    app.instantiate_contract(
        code_id,
        Addr::unchecked(ADMIN_ADDR),
        &InstantiateMsg {
            admin: ADMIN_ADDR.to_string(),
            payment_info: PaymentInfo::Cw20Payment {
                token_address: token_addr.to_string(),
                payment_amount: Uint128::new(50),
            },
        },
        &[],
        "Code ID Registry",
        None,
    )
    .unwrap();
}

fn register_cw20(
    app: &mut App,
    contract_addr: Addr,
    amount: Uint128,
    name: String,
    version: String,
    code_id: u64,
    sender: Addr,
    token_addr: Addr,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: contract_addr.to_string(),
        amount,
        msg: to_binary(&ReceiveMsg::Register {
            name,
            version,
            chain_id: CHAIN_ID.to_string(),
            code_id,
        })
        .unwrap(),
    };
    app.execute_contract(sender, token_addr, &msg, &[])
}

fn register_native(
    app: &mut App,
    contract_addr: Addr,
    amount: u128,
    denom: &str,
    name: String,
    version: String,
    code_id: u64,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Register {
        name,
        version,
        chain_id: CHAIN_ID.to_string(),
        code_id,
    };
    app.execute_contract(sender, contract_addr, &msg, &coins(amount, denom))
}

fn query_cw20_balance(app: &mut App, token_addr: Addr, addr: Addr) -> Uint128 {
    let msg = cw20_base::msg::QueryMsg::Balance {
        address: addr.to_string(),
    };
    let res: BalanceResponse = app.wrap().query_wasm_smart(token_addr, &msg).unwrap();
    res.balance
}

fn set_owner(
    app: &mut App,
    contract_addr: Addr,
    name: String,
    owner: Option<String>,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::SetOwner {
        name,
        chain_id: CHAIN_ID.to_string(),
        owner,
    };
    app.execute_contract(sender, contract_addr, &msg, &[])
}

fn unregister(
    app: &mut App,
    contract_addr: Addr,
    code_id: u64,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::Unregister {
        chain_id: CHAIN_ID.to_string(),
        code_id,
    };
    app.execute_contract(sender, contract_addr, &msg, &[])
}

fn update_config(
    app: &mut App,
    contract_addr: Addr,
    admin: Option<String>,
    payment_info: Option<PaymentInfo>,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = ExecuteMsg::UpdateConfig {
        admin,
        payment_info,
    };
    app.execute_contract(sender, contract_addr, &msg, &[])
}

fn query_get_registration(
    app: &mut App,
    contract_addr: Addr,
    name: String,
    version: Option<String>,
) -> GetRegistrationResponse {
    let msg = QueryMsg::GetRegistration {
        name,
        chain_id: CHAIN_ID.to_string(),
        version,
    };
    app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
}

fn query_info_for_code_id(
    app: &mut App,
    contract_addr: Addr,
    code_id: u64,
) -> InfoForCodeIdResponse {
    let msg = QueryMsg::InfoForCodeId {
        chain_id: CHAIN_ID.to_string(),
        code_id,
    };
    app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
}

fn query_list_registrations(
    app: &mut App,
    contract_addr: Addr,
    name: String,
) -> ListRegistrationsResponse {
    let msg = QueryMsg::ListRegistrations {
        name,
        chain_id: CHAIN_ID.to_string(),
    };
    app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
}

fn query_config(app: &mut App, contract_addr: Addr) -> Config {
    let msg = QueryMsg::Config {};
    app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
}

#[test]
fn test_register_cw20() {
    let mut app = setup_app();
    let token = create_token(&mut app);
    let contract = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let other_token = create_token(&mut app); // To be used when sending wrong token
    let name: &str = "Name";
    let version: &str = "0.0.1";
    let code_id: u64 = 1;

    // Give user address ownership over name.
    set_owner(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(USER_ADDR.to_string()),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Registering using native funds should fail.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        50,
        "ujuno",
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::InvalidPayment {});

    // Registering using wrong cw20 should fail.
    let err: ContractError = register_cw20(
        &mut app,
        contract.clone(),
        Uint128::new(50),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
        other_token,
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::UnrecognizedCw20 {});

    // Sending too little should fail.
    let err: ContractError = register_cw20(
        &mut app,
        contract.clone(),
        Uint128::new(25),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
        token.clone(),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::IncorrectPaymentAmount {});

    // Sending too much should fail.
    let err: ContractError = register_cw20(
        &mut app,
        contract.clone(),
        Uint128::new(75),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
        token.clone(),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::IncorrectPaymentAmount {});

    // Sending correct amounts should succeed.
    register_cw20(
        &mut app,
        contract.clone(),
        Uint128::new(50),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
        token.clone(),
    )
    .unwrap();

    // Check that admin now has 1050
    // It started with 1000 and now has 50 from the success
    let balance = query_cw20_balance(&mut app, token.clone(), Addr::unchecked(ADMIN_ADDR));
    assert_eq!(balance, Uint128::new(1050));

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None);
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    );
    assert_eq!(
        resp_without_version.registration,
        Registration {
            registered_by: Addr::unchecked(USER_ADDR),
            version: version.to_string(),
            code_id
        }
    );
    assert_eq!(
        resp_without_version.registration,
        resp_with_version.registration,
    );

    // Should fail with Code ID already registered.
    let err: ContractError = register_cw20(
        &mut app,
        contract,
        Uint128::new(50),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
        token,
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(
        err,
        ContractError::CodeIDAlreadyRegistered(code_id, CHAIN_ID.to_string())
    );
}

#[test]
fn test_register_native() {
    let mut app = setup_app();
    let token = create_token(&mut app);
    let pay_denom = "ujuno";
    let contract = setup_test_case(
        &mut app,
        PaymentInfo::NativePayment {
            token_denom: pay_denom.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let name: &str = "Name";
    let version: &str = "0.0.1";
    let code_id: u64 = 1;

    // Give user address ownership over name.
    set_owner(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(USER_ADDR.to_string()),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Registering using cw20 should fail.
    let err: ContractError = register_cw20(
        &mut app,
        contract.clone(),
        Uint128::new(50),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
        token,
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::InvalidPayment {});

    // Registering using wrong denom should fail.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        50,
        "uatom",
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::UnrecognizedNativeToken {});

    // Sending too little should fail.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        25,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::IncorrectPaymentAmount {});

    // Sending too much should fail.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        75,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::IncorrectPaymentAmount {});

    // Sending correct amounts should succeed.
    register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Check that admin now has 10050
    // It started with 10000 and now has 50 from the success
    let balance = app
        .wrap()
        .query_balance(Addr::unchecked(ADMIN_ADDR), pay_denom)
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(10050));

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None);
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    );
    assert_eq!(
        resp_without_version.registration,
        Registration {
            registered_by: Addr::unchecked(USER_ADDR),
            version: version.to_string(),
            code_id
        }
    );
    assert_eq!(
        resp_without_version.registration,
        resp_with_version.registration,
    );

    // Should fail with Code ID already registered.
    let err: ContractError = register_native(
        &mut app,
        contract,
        50,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(
        err,
        ContractError::CodeIDAlreadyRegistered(code_id, CHAIN_ID.to_string())
    );
}

#[test]
fn test_immutability() {
    let mut app = setup_app();
    let pay_denom = "ujuno";
    let contract = setup_test_case(
        &mut app,
        PaymentInfo::NativePayment {
            token_denom: pay_denom.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let name: &str = "Name";
    let version: &str = "0.0.1";
    let code_id: u64 = 1;

    // Give user address ownership over name.
    set_owner(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(USER_ADDR.to_string()),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Register.
    register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None);
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    );
    assert_eq!(
        resp_without_version.registration,
        Registration {
            registered_by: Addr::unchecked(USER_ADDR),
            version: version.to_string(),
            code_id
        }
    );
    assert_eq!(
        resp_without_version.registration,
        resp_with_version.registration,
    );

    // Should fail with Code ID already registered.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(
        err,
        ContractError::CodeIDAlreadyRegistered(code_id, CHAIN_ID.to_string())
    );

    // Should fail with version already registered.
    let err: ContractError = register_native(
        &mut app,
        contract,
        50,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id + 1,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(
        err,
        ContractError::VersionAlreadyRegistered(
            version.to_string(),
            name.to_string(),
            CHAIN_ID.to_string()
        )
    );
}

#[test]
fn test_set_owner() {
    let mut app = setup_app();
    let pay_denom = "ujuno";
    let contract = setup_test_case(
        &mut app,
        PaymentInfo::NativePayment {
            token_denom: pay_denom.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let name: &str = "Name";
    let version: &str = "0.0.1";
    let code_id: u64 = 1;

    // Register fails with unauthorized.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Give user address ownership over name.
    set_owner(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(USER_ADDR.to_string()),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Register succeeds.
    register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None);
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    );
    assert_eq!(
        resp_without_version.registration,
        Registration {
            registered_by: Addr::unchecked(USER_ADDR),
            version: version.to_string(),
            code_id
        }
    );
    assert_eq!(
        resp_without_version.registration,
        resp_with_version.registration,
    );

    // OTHER USER
    let new_version: &str = "0.0.2";
    let new_code_id: u64 = 2;

    // Register by other user fails with unauthorized.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        new_version.to_string(),
        new_code_id,
        Addr::unchecked(OTHER_USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Give other user address ownership over name.
    set_owner(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(OTHER_USER_ADDR.to_string()),
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Register by original user fails with unauthorized.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        new_version.to_string(),
        new_code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Register by other user succeeds.
    register_native(
        &mut app,
        contract.clone(),
        50,
        pay_denom,
        name.to_string(),
        new_version.to_string(),
        new_code_id,
        Addr::unchecked(OTHER_USER_ADDR),
    )
    .unwrap();

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None);
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(new_version.to_string()),
    );
    assert_eq!(
        resp_without_version.registration,
        Registration {
            registered_by: Addr::unchecked(OTHER_USER_ADDR),
            version: new_version.to_string(),
            code_id: new_code_id
        }
    );
    assert_eq!(
        resp_without_version.registration,
        resp_with_version.registration,
    );

    // Get all registrations and verify both exist.
    let registrations =
        query_list_registrations(&mut app, contract, name.to_string()).registrations;
    assert_eq!(
        registrations,
        vec![
            Registration {
                registered_by: Addr::unchecked(USER_ADDR),
                version: version.to_string(),
                code_id
            },
            Registration {
                registered_by: Addr::unchecked(OTHER_USER_ADDR),
                version: new_version.to_string(),
                code_id: new_code_id
            }
        ]
    )
}

#[test]
fn test_update_config() {
    let mut app = setup_app();
    let token = create_token(&mut app);
    let names = setup_test_case(
        &mut app,
        PaymentInfo::Cw20Payment {
            token_address: token.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let other_token = create_token(&mut app); // To be used when updating payment token

    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR),
            payment_info: PaymentInfo::Cw20Payment {
                token_address: token.to_string(),
                payment_amount: Uint128::new(50)
            }
        }
    );

    // Update config as non admin fails
    let err: ContractError = update_config(
        &mut app,
        names.clone(),
        Some(other_token.to_string()),
        Some(PaymentInfo::NativePayment {
            token_denom: "ujuno".to_string(),
            payment_amount: Uint128::new(50),
        }),
        Addr::unchecked(OTHER_USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Ensure config stayed the same.
    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR),
            payment_info: PaymentInfo::Cw20Payment {
                token_address: token.to_string(),
                payment_amount: Uint128::new(50)
            }
        }
    );

    // Update config as admin
    update_config(
        &mut app,
        names.clone(),
        Some(OTHER_USER_ADDR.to_string()),
        Some(PaymentInfo::NativePayment {
            token_denom: "ujuno".to_string(),
            payment_amount: Uint128::new(25),
        }),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(OTHER_USER_ADDR),
            payment_info: PaymentInfo::NativePayment {
                token_denom: "ujuno".to_string(),
                payment_amount: Uint128::new(25)
            }
        }
    );

    // Update one config value but not the others

    // Only admin
    update_config(
        &mut app,
        names.clone(),
        Some(ADMIN_ADDR.to_string()),
        None,
        Addr::unchecked(OTHER_USER_ADDR),
    )
    .unwrap();

    let config = query_config(&mut app, names.clone());
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR), // Only this has changed
            payment_info: PaymentInfo::NativePayment {
                token_denom: "ujuno".to_string(),
                payment_amount: Uint128::new(25)
            }
        }
    );

    // Only payment info
    update_config(
        &mut app,
        names.clone(),
        None,
        Some(PaymentInfo::NativePayment {
            token_denom: "uatom".to_string(),
            payment_amount: Uint128::new(50),
        }),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    let config = query_config(&mut app, names);
    assert_eq!(
        config,
        Config {
            admin: Addr::unchecked(ADMIN_ADDR),
            payment_info: PaymentInfo::NativePayment {
                token_denom: "uatom".to_string(),
                payment_amount: Uint128::new(50)
            }
        }
    );
}
