use cosmwasm_std::{
    entry_point, Deps, DepsMut, Empty, Env, IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, QueryResponse, Response
};
use cw2::{get_contract_version, set_contract_version};
use shared::{
    msg::contract::warehouse::{ExecuteMsg, GroupInfo, InfoResp, QueryMsg}, response::{QueryResponseExt, ResponseBuilder},
};
use anyhow::Result;

use crate::state::{ibc::IbcChannelKind, State, StateContext};
// version info for migration info
const CONTRACT_NAME: &str = "warehouse";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> Result<Response> {
    let (state, mut ctx) = StateContext::new(deps, env)?;

    match msg {
        ExecuteMsg::AddProduct { product} => {
            state.add_product(&mut ctx, info.sender, product)?;
        },
        ExecuteMsg::ShipGroup { group_id } => {
            state.ship_group(&mut ctx, info.sender, group_id)?;
        }
    }

    Ok(ctx.response.into_response())
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<QueryResponse> {
    let (state, store) = State::new(deps, env)?;

    match msg {
        QueryMsg::ListProducts {owner, limit, start_after} => {
            let products = state.list_products(store, owner, limit, start_after)?;
            products.query_result()
        }
        QueryMsg::ListGroups {owner, limit, start_after} => {
            let groups = state.list_groups(store, owner, limit, start_after)?;
            groups.query_result()
        },
        QueryMsg::GetPurchases {ids} => {
            let purchases = state.get_purchases(store, ids)?;
            purchases.query_result()
        },
        QueryMsg::GetProducts {ids} => {
            let products = state.get_products(store, ids)?;
            products.query_result()
        }
        QueryMsg::GetGroups {ids} => {
            ids
                .into_iter()
                .map(|id| state.get_group_info(store, id))
                .collect::<Result<Vec<GroupInfo>>>()?
                .query_result()
        }
        QueryMsg::Info {  } => {
            let ibc_payment_channel = state.get_ibc_channel(store, IbcChannelKind::Payment).ok();
            let info = InfoResp {
                ibc_payment_channel
            };
            info.query_result()
        }
    }
}

/// Handles the `OpenInit` and `OpenTry` parts of the IBC handshake.
#[entry_point]
pub fn ibc_channel_open(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse> {
    let (state, _) = StateContext::new(deps, env)?;
    state.handle_ibc_channel_open(msg)?;
    Ok(None)
}

#[entry_point]
pub fn ibc_channel_connect(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse> {
    let (state, mut ctx) = StateContext::new(deps, env)?;
    state.handle_ibc_channel_connect(&mut ctx, msg)?;
    Ok(ctx.response.into_ibc_response())
}

#[entry_point]
pub fn ibc_channel_close(
    deps: DepsMut,
    env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse> {
    let (state, mut ctx) = StateContext::new(deps, env)?;
    state.handle_ibc_channel_close(&mut ctx, msg)?;
    Ok(ctx.response.into_ibc_response())
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse> {
    let (state, mut ctx) = StateContext::new(deps, env)?;
    state.handle_ibc_packet_receive(&mut ctx, msg)?;

    Ok(ctx.response.into_ibc_recv_response_success())


    // Regardless of if our processing of this packet works we need to
    // commit an ACK to the chain. As such, we wrap all handling logic
    // in a seprate function and on error write out an error ack.
    // TODO: reconsider https://github.com/CosmWasm/cosmwasm/blob/main/IBC.md#acknowledging-errors
    // match resp {
    //     Ok(_) => Ok(ctx.response.into_ibc_recv_response_success()),
    //     Err(error) => Ok(ResponseBuilder::new(get_contract_version(ctx.store)?)
    //         .into_ibc_recv_response_fail(error)),
    // }
}

#[entry_point]
pub fn ibc_packet_ack(deps: DepsMut, env: Env, ack: IbcPacketAckMsg) -> Result<IbcBasicResponse> {
    let (state, ctx) = StateContext::new(deps, env)?;
    state.handle_ibc_packet_ack(ack)?;
    Ok(ctx.response.into_ibc_response())
}

#[entry_point]
pub fn ibc_packet_timeout(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse> {
    let (state, ctx) = StateContext::new(deps, env)?;
    state.handle_ibc_packet_timeout(msg)?;
    Ok(ctx.response.into_ibc_response())
}
