use crate::msg::{
    ExecuteMsg, GetRegistrationResponse, InfoForCodeIdResponse, InstantiateMsg,
    ListRegistrationsResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, PaymentInfo, Registration};
use crate::ContractError;
use anyhow::Result as AnyResult;
use cosmwasm_std::{coin, coins, to_binary, Addr, Coin, Empty, StdResult, Uint128};
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
    // Pass token address as denom to avoid clippy argument count warning.
    funds: Coin,
    name: String,
    version: String,
    code_id: u64,
    sender: Addr,
) -> AnyResult<AppResponse> {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: contract_addr.to_string(),
        amount: funds.amount,
        msg: to_binary(&ReceiveMsg::Register {
            name,
            version,
            chain_id: CHAIN_ID.to_string(),
            code_id,
        })
        .unwrap(),
    };
    app.execute_contract(sender, Addr::unchecked(funds.denom), &msg, &[])
}

fn register_native(
    app: &mut App,
    contract_addr: Addr,
    funds: Vec<Coin>,
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
    app.execute_contract(sender, contract_addr, &msg, &funds)
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
) -> StdResult<GetRegistrationResponse> {
    let msg = QueryMsg::GetRegistration {
        name,
        chain_id: CHAIN_ID.to_string(),
        version,
    };
    app.wrap().query_wasm_smart(contract_addr, &msg)
}

fn query_info_for_code_id(
    app: &mut App,
    contract_addr: Addr,
    code_id: u64,
) -> StdResult<InfoForCodeIdResponse> {
    let msg = QueryMsg::InfoForCodeId {
        chain_id: CHAIN_ID.to_string(),
        code_id,
    };
    app.wrap().query_wasm_smart(contract_addr, &msg)
}

fn query_list_registrations(
    app: &mut App,
    contract_addr: Addr,
    name: String,
) -> StdResult<ListRegistrationsResponse> {
    let msg = QueryMsg::ListRegistrations {
        name,
        chain_id: CHAIN_ID.to_string(),
    };
    app.wrap().query_wasm_smart(contract_addr, &msg)
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
        coins(50, "ujuno"),
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
        coin(50, other_token),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::UnrecognizedCw20 {});

    // Sending too little should fail.
    let err: ContractError = register_cw20(
        &mut app,
        contract.clone(),
        coin(25, token.to_string()),
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
    let err: ContractError = register_cw20(
        &mut app,
        contract.clone(),
        coin(75, token.to_string()),
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
    register_cw20(
        &mut app,
        contract.clone(),
        coin(50, token.to_string()),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Check that admin now has 1050
    // It started with 1000 and now has 50 from the success
    let balance = query_cw20_balance(&mut app, token.clone(), Addr::unchecked(ADMIN_ADDR));
    assert_eq!(balance, Uint128::new(1050));

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None).unwrap();
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    )
    .unwrap();
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
        coin(50, token),
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
        coin(50, token),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::InvalidPayment {});

    // Registering using wrong denom should fail.
    let err: ContractError = register_native(
        &mut app,
        contract.clone(),
        coins(50, "uatom"),
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
        coins(25, pay_denom),
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
        coins(75, pay_denom),
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
        coins(50, pay_denom),
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
        query_get_registration(&mut app, contract.clone(), name.to_string(), None).unwrap();
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    )
    .unwrap();
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
        coins(50, pay_denom),
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
        coins(50, pay_denom),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None).unwrap();
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    )
    .unwrap();
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
        coins(50, pay_denom),
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
        coins(50, pay_denom),
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
        coins(50, pay_denom),
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
        coins(50, pay_denom),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None).unwrap();
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    )
    .unwrap();
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
        coins(50, pay_denom),
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
        coins(50, pay_denom),
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
        coins(50, pay_denom),
        name.to_string(),
        new_version.to_string(),
        new_code_id,
        Addr::unchecked(OTHER_USER_ADDR),
    )
    .unwrap();

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None).unwrap();
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(new_version.to_string()),
    )
    .unwrap();
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
    let registrations = query_list_registrations(&mut app, contract.clone(), name.to_string())
        .unwrap()
        .registrations;
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
    );

    // Get info for each code ID.
    let info = query_info_for_code_id(&mut app, contract.clone(), code_id).unwrap();
    assert_eq!(
        info,
        InfoForCodeIdResponse {
            registered_by: Addr::unchecked(USER_ADDR),
            name: name.to_string(),
            version: version.to_string()
        }
    );
    let new_info = query_info_for_code_id(&mut app, contract, new_code_id).unwrap();
    assert_eq!(
        new_info,
        InfoForCodeIdResponse {
            registered_by: Addr::unchecked(OTHER_USER_ADDR),
            name: name.to_string(),
            version: new_version.to_string()
        }
    );
}

#[test]
fn test_unregister() {
    let mut app = setup_app();
    let pay_denom = "ujuno";
    let contract = setup_test_case(
        &mut app,
        PaymentInfo::NativePayment {
            token_denom: pay_denom.to_string(),
            payment_amount: Uint128::new(50),
        },
    );
    let name: &str = "Name1";
    let version1: &str = "0.0.1";
    let code_id1: u64 = 1;
    let version2: &str = "0.0.2";
    let code_id2: u64 = 2;
    let version3: &str = "0.0.3";
    let code_id3: u64 = 3;

    let reg1 = Registration {
        registered_by: Addr::unchecked(USER_ADDR),
        version: version1.to_string(),
        code_id: code_id1,
    };
    let reg2 = Registration {
        registered_by: Addr::unchecked(USER_ADDR),
        version: version2.to_string(),
        code_id: code_id2,
    };
    let reg3 = Registration {
        registered_by: Addr::unchecked(USER_ADDR),
        version: version3.to_string(),
        code_id: code_id3,
    };

    // Give user address ownership over name.
    set_owner(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(USER_ADDR.to_string()),
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Register 1 succeeds.
    register_native(
        &mut app,
        contract.clone(),
        coins(50, pay_denom),
        name.to_string(),
        version1.to_string(),
        code_id1,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Register 2 succeeds.
    register_native(
        &mut app,
        contract.clone(),
        coins(50, pay_denom),
        name.to_string(),
        version2.to_string(),
        code_id2,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Register 3 succeeds.
    register_native(
        &mut app,
        contract.clone(),
        coins(50, pay_denom),
        name.to_string(),
        version3.to_string(),
        code_id3,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Get all registrations and verify all exist.
    let registrations = query_list_registrations(&mut app, contract.clone(), name.to_string())
        .unwrap()
        .registrations;
    assert_eq!(
        registrations,
        vec![reg1.clone(), reg2.clone(), reg3.clone()]
    );

    // Get latest and ensure it is 3.
    let latest_registration =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None)
            .unwrap()
            .registration;
    assert_eq!(latest_registration, reg3);

    // Attempt unregister 3 by user but fail because not admin.
    let err: ContractError = unregister(
        &mut app,
        contract.clone(),
        code_id3,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap_err()
    .downcast()
    .unwrap();
    assert_eq!(err, ContractError::Unauthorized {});

    // Unregister 3
    unregister(
        &mut app,
        contract.clone(),
        code_id3,
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Attempt to get info for 3 and expect not found.
    let err = query_info_for_code_id(&mut app, contract.clone(), code_id3).unwrap_err();
    assert!(err
        .to_string()
        .contains(&ContractError::NotFound {}.to_string()));
    // Attempt to get registration for 3 and expect not found.
    let err = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version3.to_string()),
    )
    .unwrap_err();
    assert!(err
        .to_string()
        .contains(&ContractError::NotFound {}.to_string()));

    // Get latest and ensure it is 2.
    let latest_registration =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None)
            .unwrap()
            .registration;
    assert_eq!(latest_registration, reg2);

    // Get all registrations and verify only 1 and 2 exist.
    let registrations = query_list_registrations(&mut app, contract.clone(), name.to_string())
        .unwrap()
        .registrations;
    assert_eq!(registrations, vec![reg1, reg2]);

    // Unregister 1 and 2
    unregister(
        &mut app,
        contract.clone(),
        code_id1,
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();
    unregister(
        &mut app,
        contract.clone(),
        code_id2,
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Expect not found when attempting to get latest.
    let err =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None).unwrap_err();
    assert!(err
        .to_string()
        .contains(&ContractError::NotFound {}.to_string()));

    // Get all registrations and expect not found.
    let err = query_list_registrations(&mut app, contract, name.to_string()).unwrap_err();
    assert!(err
        .to_string()
        .contains(&ContractError::NotFound {}.to_string()));
}

#[test]
fn test_mutable_after_unregister() {
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
        coins(50, pay_denom),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Check registration with and without version.
    let resp_without_version =
        query_get_registration(&mut app, contract.clone(), name.to_string(), None).unwrap();
    let resp_with_version = query_get_registration(
        &mut app,
        contract.clone(),
        name.to_string(),
        Some(version.to_string()),
    )
    .unwrap();
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
        coins(50, pay_denom),
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
        contract.clone(),
        coins(50, pay_denom),
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

    // Unregister
    unregister(
        &mut app,
        contract.clone(),
        code_id,
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Should NOT fail with Code ID already registered.
    register_native(
        &mut app,
        contract.clone(),
        coins(50, pay_denom),
        name.to_string(),
        version.to_string(),
        code_id,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Unregister
    unregister(
        &mut app,
        contract.clone(),
        code_id,
        Addr::unchecked(ADMIN_ADDR),
    )
    .unwrap();

    // Should NOT fail with version already registered.
    register_native(
        &mut app,
        contract.clone(),
        coins(50, pay_denom),
        name.to_string(),
        version.to_string(),
        code_id + 1,
        Addr::unchecked(USER_ADDR),
    )
    .unwrap();

    // Get info for new code ID.
    let info = query_info_for_code_id(&mut app, contract, code_id + 1).unwrap();
    assert_eq!(
        info,
        InfoForCodeIdResponse {
            registered_by: Addr::unchecked(USER_ADDR),
            name: name.to_string(),
            version: version.to_string()
        }
    );
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
