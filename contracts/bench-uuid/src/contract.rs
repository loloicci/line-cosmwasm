use cosmwasm_std::{
    entry_point, new_uuid, new_uuid_native, Api, DepsMut, Env, MessageInfo, Response, StdResult, Storage, Uuid,
};

use crate::msg::{ExecuteMsg, InstantiateMsg};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    let uuid = match msg {
        ExecuteMsg::Wasm {} => try_wasm_uuid(&env, deps.storage)?,
        ExecuteMsg::Api {} => try_api_uuid(&env, deps.storage, deps.api)?,
    };
    Ok(Response::default().add_attribute("uuid", format!("{:?}", uuid.as_slice())))
}

fn try_wasm_uuid(env: &Env, storage: &mut dyn Storage) -> StdResult<Uuid> {
    new_uuid_native(env, storage)
}

fn try_api_uuid(env: &Env, storage: &mut dyn Storage, api: &dyn Api) -> StdResult<Uuid> {
    new_uuid(env, storage, api)
}
