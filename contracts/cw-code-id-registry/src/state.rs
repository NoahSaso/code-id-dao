use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Config {
    /// Admin receives fees, can register anything, and set owners to allow
    /// future registration.
    pub admin: Addr,
    pub payment_info: PaymentInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Registration {
    pub registered_by: Addr,
    pub version: String,
    pub code_id: u64,
    pub checksum: String,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Map (name, chain_id, version) to a code_id.
pub const NAME_CHAIN_ID_VERSION_TO_CODE_ID: Map<(String, String, String), u64> =
    Map::new("name_chain_id_version_to_code_id");
/// Map (name, chain_id) to the owner.
pub const NAME_CHAIN_ID_TO_OWNER: Map<(String, String), Addr> = Map::new("owner");
/// Map (chain_id, code_id) to the contract name.
pub const CHAIN_ID_CODE_ID_TO_NAME: Map<(String, u64), String> =
    Map::new("chain_id_code_id_to_name");
/// Map (name, chain_id, code_id) to the registration.
pub const NAME_CHAIN_ID_CODE_ID_TO_REGISTRATION: Map<(String, String, u64), Registration> =
    Map::new("name_chain_id_code_id_to_registration");
