use cosmwasm_std::{
    entry_point, make_dependencies, DepsMut, Env, MessageInfo, Response, StdResult, release_buffer, Empty, new_uuid_original, new_uuid_api, new_uuid_api_separate, new_uuid_api_concat, new_uuid_native, new_uuid_native_concat,
    BlockInfo, Timestamp, ContractInfo, Addr, Storage, to_vec,
};

use crate::msg::InstantiateMsg;


pub fn mock_env() -> Env {
    Env {
        block: BlockInfo {
            height: 12_345,
            time: Timestamp::from_nanos(1_571_797_419_879_305_533),
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        transaction: None,
        contract: ContractInfo {
            address: Addr::unchecked("link14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sgf2vn8"),
        },
    }
}

const CONTRACT_UUID_SEQ_NUM_KEY: &[u8] = b"contract_uuid_seq_num";
//let canonical_addr = [0xad, 0xe4, 0xa5, 0xf5, 0x80, 0x3a, 0x43, 0x98, 0x35, 0xc6, 0x36, 0x39, 0x5a, 0x8d, 0x64, 0x8d, 0xee, 0x57, 0xb2, 0xfc, 0x90, 0xd9, 0x8d, 0xc1, 0x7f, 0xa8, 0x87, 0x15, 0x9b, 0x69, 0x63, 0x8b];

// hack: instantiate uuid seq
fn init_seq_num(storage: &mut dyn Storage) {
    let seq_num: u16 = 0;
    storage.set(CONTRACT_UUID_SEQ_NUM_KEY, &(to_vec(&seq_num).unwrap()));
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    init_seq_num(deps.storage);
    Ok(Response::default())
}

#[no_mangle]
pub extern "C" fn do_init_seq() {
    let mut deps = make_dependencies::<Empty>();
    init_seq_num(&mut deps.storage);
}

#[no_mangle]
pub extern "C" fn make_deps_and_output() -> u32 {
    let _deps = make_dependencies::<Empty>();
    let output = b"output";
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn uuid_original() -> u32 {
    let mut deps = make_dependencies::<Empty>();
    let output = new_uuid_original(&mock_env(), &mut deps.storage, &deps.api).unwrap();
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn uuid_api() -> u32 {
    let mut deps = make_dependencies::<Empty>();
    let output = new_uuid_api(&mock_env(), &mut deps.storage, &deps.api).unwrap();
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn uuid_api_separate() -> u32 {
    let mut deps = make_dependencies::<Empty>();
    let output = new_uuid_api_separate(&mock_env(), &mut deps.storage, &deps.api).unwrap();
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn uuid_api_concat() -> u32 {
    let mut deps = make_dependencies::<Empty>();
    let output = new_uuid_api_concat(&mock_env(), &mut deps.storage, &deps.api).unwrap();
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn uuid_native() -> u32 {
    let mut deps = make_dependencies::<Empty>();
    let output = new_uuid_native(&mock_env(), &mut deps.storage).unwrap();
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn uuid_native_concat() -> u32 {
    let mut deps = make_dependencies::<Empty>();
    let output = new_uuid_native_concat(&mock_env(), &mut deps.storage).unwrap();
    release_buffer(output.to_vec()) as u32
}
