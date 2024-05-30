use cosmwasm_std::{to_json_binary, Addr, Coin, Decimal256, IbcMsg, IbcTimeout, Storage, Uint128};
use cw_storage_plus::{Bound, Item, Map};
use shared::{ibc::TIMEOUT_SECONDS, msg::{contract::{payment::{IbcExecuteMsg as PaymentIbcExecuteMsg, Refund}, warehouse::{event::{AddProductEvent, PurchaseEvent}, NewProduct}}, product::{Product, ProductId}, purchase::{Purchase, PurchaseId}}};
use anyhow::Result;

use super::{ibc::IbcChannelKind, State, StateContext};

const PURCHASE_ID: Item<PurchaseId> = Item::new("purchase-index");
const PURCHASES: Map<PurchaseId, Purchase> = Map::new("purchases");

impl State<'_> {
    pub fn get_purchases(&self, store: &dyn Storage, purchase_ids: Vec<PurchaseId>) -> Result<Vec<Purchase>> {
        let purchases = purchase_ids
            .into_iter()
            .map(|id| PURCHASES.load(store, id))
            .collect::<Result<Vec<Purchase>, _>>()?;

        Ok(purchases)
    }

    pub fn make_purchase(&self, ctx: &mut StateContext, spender: String, product_id: ProductId, quantity: u32, fees: Uint128) -> Result<PurchaseId> {
        let product = self.get_product(ctx.store, product_id)?;
        let fees = Decimal256::from_ratio(fees.u128(), 1u32);
        let cost = product.price * Decimal256::from_ratio(quantity, 1u32);
        if fees < cost {
            anyhow::bail!("fees must cover the cost of the purchase");
        }

        self.remove_product_stock(ctx, product_id, quantity)?;

        let id = PURCHASE_ID.may_load(ctx.store)?.unwrap_or_default();
        PURCHASE_ID.save(ctx.store, &(id + 1))?;

        let group_id = self.add_purchase_to_group(ctx, id, product_id)?;

        let purchase = Purchase {
            id,
            product_id,
            quantity,
            spender,
            group_id
        };


        PURCHASES.save(ctx.store, id, &purchase)?;


        ctx.response_mut()
            .add_event(PurchaseEvent{ purchase});

        Ok(id)
    }

    pub fn try_get_purchase(&self, store: &dyn Storage, id: PurchaseId) -> Result<Option<Purchase>> {
        PURCHASES.may_load(store, id).map_err(|e| e.into())
    }

    pub fn remove_purchase(&self, ctx: &mut StateContext, id: PurchaseId) -> Result<()> {
        let purchase = PURCHASES.load(ctx.store, id)?;
        let product = self.get_product(ctx.store, purchase.product_id)?;

        self.remove_purchase_from_group(ctx, purchase.id, purchase.product_id)?;

        PURCHASES.remove(ctx.store, id);

        self.add_product_stock(ctx, product.id, purchase.quantity)?;

        // refund the purchase to the original spender (not the nft owner)
        let refund = product.price * Decimal256::from_ratio(purchase.quantity, 1u32);

        let msg = PaymentIbcExecuteMsg::Refund { refunds: vec![
            Refund {
                recipient: purchase.spender,
                amount: refund.to_uint_floor().to_string().parse()?
            }
        ]};

        let channel_id = self
            .get_ibc_channel(ctx.store, IbcChannelKind::Payment)?
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