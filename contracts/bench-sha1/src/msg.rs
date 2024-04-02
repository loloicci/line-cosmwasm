use cosmwasm_schema::cw_serde;

/// A placeholder where we don't take any input
#[cw_serde]
pub struct InstantiateMsg {
    pub pre: String
}
