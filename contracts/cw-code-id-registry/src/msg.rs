use crate::state::{PaymentInfo, Registration};
use cosmwasm_std::Addr;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateMsg {
    pub admin: String,
    pub payment_info: PaymentInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecuteMsg {
    /// Receive payment to register when payment info is a CW20.
    Receive(Cw20ReceiveMsg),
    /// Receive payment to register when payment info is native.
    Register {
        name: String,
        version: String,
        chain_id: String,
        code_id: u64,
        checksum: String,
    },
    /// Set owner for registration.
    SetOwner {
        name: String,
        chain_id: String,
        owner: Option<String>,
    },
    /// Allow admin to unregister code IDs.
    Unregister { name: String, chain_id: String, code_id: u64 },
    /// Update config.
    UpdateConfig {
        admin: Option<String>,
        payment_info: Option<PaymentInfo>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ReceiveMsg {
    // Receive payment to register when payment info is a CW20.
    Register {
        name: String,
        version: String,
        chain_id: String,
        code_id: u64,
        checksum: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryMsg {
    Config {},
    /// If version provided, tries to find given version. Otherwise returns
    /// the latest version registered.
    GetRegistration {
        name: String,
        chain_id: String,
        version: Option<String>,
    },
    InfoForCodeId {
        chain_id: String,
        code_id: u64,
    },
    ListRegistrations {
        name: String,
        chain_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct GetRegistrationResponse {
    pub registration: Registration,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InfoForCodeIdResponse {
    pub registered_by: Addr,
    pub name: String,
    pub version: String,
    pub checksum: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ListRegistrationsResponse {
    pub registrations: Vec<Registration>,
}
