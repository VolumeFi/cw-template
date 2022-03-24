use std::convert::TryInto;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Order::Ascending;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Order};
use cw2::set_contract_version;
use cw_storage_plus::U32Key;
use crate::ContractError::{Std, Unauthorized};

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Record, State, RECORD, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:{{project-name}}";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = State {
        count: msg.count,
        owner: info.sender.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("count", msg.count.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Increment {} => try_increment(deps),
        ExecuteMsg::Reset { count } => try_reset(deps, info, count),
        ExecuteMsg::Add { number } => try_add(deps, number),
        ExecuteMsg::Remove { index } => try_remove(deps, index),
        ExecuteMsg::RemoveItem { number } => try_remove_item(deps, number),
    }
}

pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.count += 1;
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_increment"))
}
pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if info.sender != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.count = count;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "reset"))
}

pub fn try_add(deps: DepsMut, number: i32) -> Result<Response, ContractError> {
    let keys = RECORD.keys(deps.storage, None, None, Ascending);
    let last_key = keys.last();
    if last_key.is_none() {
        RECORD.save(
            deps.storage,
            U32Key::from(0),
            &Record { number },
        )?;
        Ok(Response::new().add_attribute("method", "add").add_attribute("new_key", 0.to_string()))
    } else {
        let new_key = u32::from_be_bytes(last_key.unwrap().as_slice().try_into().unwrap()) + 1;
        RECORD.save(
            deps.storage,
            U32Key::from(new_key),
            &Record { number },
        )?;
        Ok(Response::new().add_attribute("method", "add"))
    }
}

pub fn try_remove(deps: DepsMut, index: u32) -> Result<Response, ContractError> {
    RECORD.remove(deps.storage, U32Key::from(index));
    if RECORD.has(deps.storage, U32Key::from(index)) {
        return Err(Unauthorized{});
    }
    Ok(Response::new().add_attribute("method", "remove").add_attribute("key", index.to_string()))
}

pub fn try_remove_item(deps: DepsMut, number: i32) -> Result<Response, ContractError> {
    let keys = RECORD.keys(deps.storage, None, None, Order::Ascending);
    let mut remove_keys = Vec::new();
    for key in keys {
        let record = RECORD.load(deps.storage, U32Key::from(key.clone()))?;
        if record.number == number {
            remove_keys.push(U32Key::from(key));
        }
    }
    for key in remove_keys {
        RECORD.remove(deps.storage, key);
    }
    Ok(Response::new().add_attribute("method", "remove_item"))
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
        QueryMsg::GetRecord { index } => to_binary(&query_record(deps, index)?),
    }
}

fn query_count(deps: Deps) -> StdResult<CountResponse> {
    let keys = RECORD.keys(deps.storage, None, None, Ascending);
    Ok(CountResponse { count: keys.count() as i32 })
}

fn query_record(deps: Deps, index: u32) -> StdResult<Record> {
    let record_list = RECORD.may_load(deps.storage, U32Key::from(index))?;
    let result = record_list.unwrap_or(Record { number: 0 });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balances, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, to_vec};

    #[test]
    fn increment() {
        let mut deps = mock_dependencies_with_balances(&[]);

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        for i in 0..2000 {
            let msg = ExecuteMsg::Add { number: i * 2 };
            let result = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        }
        // let msg = ExecuteMsg::Remove { index: 5 };
        // execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::RemoveItem { number: 4 };
        execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        for i in 0..2000 {
            let res = query(deps.as_ref(), mock_env(), QueryMsg::GetRecord { index: i }).unwrap();
            let value: Record = from_binary(&res).unwrap();
            println!("{}", value.number);
        }
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();

        // println!("{}", value.count);
    }

    #[test]
    fn test() {
        // let mut last_key = vec![0x00u8, 0x00u8, 0x00u8];
        // if last_key.len() < 4usize {
        //     last_key = [vec![0; 4 - last_key.len()], last_key].concat();
        // }
        // println!("{}", i32::from_be_bytes(Binary::from(last_key.clone()).to_array().unwrap()) + 1)

        // last_key = [vec![0; 4 - last_key.len()], last_key].concat();
        // println!("{}", i32::from_be_bytes(Binary::from(last_key.clone()).to_array().unwrap()) + 1)

        // let res = to_binary(&0xffi32).unwrap().as_slice().len();
        let last_key = U32Key::from(255);
        let w_key = last_key.wrapped;
        let value = u32::from_be_bytes(w_key.as_slice().try_into().unwrap());
        print!("{}", value);
        // println!("{}", w_key[0]);
        // println!("{}", w_key[1]);
        // println!("{}", w_key[2]);
        // println!("{}", w_key[3]);
        // println!("{}", w_key.len());
        // println!("{}", i32::from_be_bytes(res.to_array().unwrap()))
    }
}
