use cosmwasm_std::{
    entry_point, make_dependencies, DepsMut, Env, MessageInfo, Response, StdResult, consume_region, Region, release_buffer, Storage, Api, Empty,
};
use sha1::{Sha1, Digest};
use uuid::Uuid;

use crate::msg::InstantiateMsg;

const PRE: &[u8] = b"pre";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let value = &msg.pre.as_bytes();
    deps.storage.set(PRE, &value);
    Ok(Response::default())
}

#[no_mangle]
pub extern "C" fn make_deps_read_input_write_output(input: u32) -> u32 {
    let deps = make_dependencies::<Empty>();
    let mut bytes = deps.storage.get(PRE).unwrap();

    bytes.append(unsafe { &mut consume_region(input as *mut Region) });

    let mut output = [0; 16];
    output.copy_from_slice(&bytes[..16]);

    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn sha1_raw(input: u32) -> u32 {
    let deps = make_dependencies::<Empty>();
    let mut bytes = deps.storage.get(PRE).unwrap();

    bytes.append(unsafe { &mut consume_region(input as *mut Region) });

    let mut hasher = Sha1::new();

    hasher.update(Uuid::NAMESPACE_OID.as_bytes());
    hasher.update(bytes);

    let buffer = hasher.finalize();

    let output = &buffer[..16];
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn sha1_raw_twice(input: u32) -> u32 {
    let deps = make_dependencies::<Empty>();
    let mut bytes = deps.storage.get(PRE).unwrap();

    bytes.append(unsafe { &mut consume_region(input as *mut Region) });

    let mut hasher = Sha1::new();

    hasher.update(Uuid::NAMESPACE_OID.as_bytes());
    hasher.update(bytes.clone());
    hasher.update(bytes);

    let buffer = hasher.finalize();

    let output = &buffer[..16];
    release_buffer(output.to_vec()) as u32
}


#[no_mangle]
pub extern "C" fn sha1_api(input: u32) -> u32 {
    let deps = make_dependencies::<Empty>();
    let mut bytes = deps.storage.get(PRE).unwrap();

    bytes.append(unsafe { &mut consume_region(input as *mut Region) });

    let buffer = deps.api.sha1_calculate(&[Uuid::NAMESPACE_OID.as_bytes(), &bytes]).unwrap();

    let output = &buffer[..16];
    release_buffer(output.to_vec()) as u32
}

#[no_mangle]
pub extern "C" fn sha1_api_twice(input: u32) -> u32 {
    let deps = make_dependencies::<Empty>();
    let mut bytes = deps.storage.get(PRE).unwrap();

    bytes.append(unsafe { &mut consume_region(input as *mut Region) });

    let buffer = deps.api.sha1_calculate(&[Uuid::NAMESPACE_OID.as_bytes(), &bytes, &bytes]).unwrap();

    let output = &buffer[..16];
    release_buffer(output.to_vec()) as u32
}
