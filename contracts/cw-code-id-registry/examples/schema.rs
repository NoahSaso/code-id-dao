use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, export_schema_with_title, remove_schemas, schema_for};

use cw_code_id_registry::{
    msg::{
        ExecuteMsg, GetRegistrationResponse, InfoForCodeIdResponse, InstantiateMsg,
        ListRegistrationsResponse, QueryMsg, ReceiveMsg,
    },
    state::{Config, PaymentInfo, Registration},
};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ReceiveMsg), &out_dir);
    export_schema(&schema_for!(PaymentInfo), &out_dir);
    export_schema(&schema_for!(Registration), &out_dir);
    export_schema(&schema_for!(GetRegistrationResponse), &out_dir);
    export_schema(&schema_for!(InfoForCodeIdResponse), &out_dir);
    export_schema(&schema_for!(ListRegistrationsResponse), &out_dir);

    // Auto TS code generation expects the query return type as QueryNameResponse
    // Here we map query responses to the correct name
    export_schema_with_title(&schema_for!(Config), &out_dir, "ConfigResponse");
}
