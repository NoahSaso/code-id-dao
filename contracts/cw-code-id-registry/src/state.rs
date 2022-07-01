use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentInfo {
    None {},
    NativePayment {
        token_denom: String,
        payment_amount: Uint128,
    },
    Cw20Payment {
        token_address: String,
        payment_amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    /// Admin receives fees, can register anything, and set owners to allow
    /// future registration.
    pub admin: Addr,
    pub payment_info: PaymentInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Registration {
    pub registered_by: Addr,
    pub version: String,
    pub code_id: u64,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Map (name, chain_id) to all registrations.
pub const ALL_REGISTRATIONS: Map<(String, String), Vec<Registration>> =
    Map::new("all_registrations");
/// Map (name, chain_id, version) to a registration.
pub const VERSION_REGISTRATION: Map<(String, String, String), Registration> =
    Map::new("version_registration");
/// Map (name, chain_id) to latest registration.
pub const LATEST_REGISTRATION: Map<(String, String), Registration> =
    Map::new("latest_registration");
/// Map (name, chain_id) to the owner.
pub const NAME_CHAIN_ID_TO_OWNER: Map<(String, String), Addr> = Map::new("owner");
/// Map (chain_id, code_id) to the contract name.
pub const CHAIN_ID_CODE_ID_TO_NAME: Map<(String, u64), String> =
    Map::new("chain_id_code_id_to_name");
