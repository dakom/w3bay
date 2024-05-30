use cosmwasm_std::{to_json_binary, Coin, IbcMsg, IbcTimeout, MessageInfo, Storage};
use cw_storage_plus::{Bound, Map};
use shared::{ibc::TIMEOUT_SECONDS, msg::{contract::warehouse::{event::AddProductEvent, NewProduct}, product::{Product, ProductId}, purchase::{Purchase, PurchaseId}}};
use anyhow::{Result, anyhow};

use super::{State, StateContext};

const PURCHASES: Map<PurchaseId, Purchase> = Map::new("purchases");

pub const PURCHASE_DENOM: &str = "ukuji";

impl State<'_> {
    pub fn purchase_send(&self, ctx: &mut StateContext, owner: String, info: MessageInfo, product_id: ProductId, quantity: u32) -> Result<()> {
        // would be nice to use Interchain Queries to early-exit if there's not enough funds
        // it's just an optimization though, since the purchase should always be confirmed in the warehouse last-minute
        // and we should handle failures in the ack to return funds to the user if IBC fails anyway

        let fees = info.funds.iter().find_map(|coin| {
            if coin.denom == PURCHASE_DENOM {
                Some(coin.amount)
            } else {
                None
            }
        }).ok_or_else(|| anyhow!(format!("must send {} to purchase", PURCHASE_DENOM)))?;

        let msg = shared::msg::contract::warehouse::IbcExecuteMsg::Purchase {
            owner,
            spender: info.sender.to_string(),
            fees,
            product_id,
            quantity,
        };

        // outbound IBC message, where packet is then received on other chain
        let channel_id = self
            .get_ibc_channel(ctx.store)?
            .endpoint
            .channel_id;

        ctx.response_mut().add_message(IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&msg)?,
            timeout: IbcTimeout::with_timestamp(self.env.block.time.plus_seconds(TIMEOUT_SECONDS)),
        });

        Ok(())
    }
}