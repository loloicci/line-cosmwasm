use cosmwasm_schema::write_api;

use bench_uuid::msg::{InstantiateMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
    }
}
