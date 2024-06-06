
use cosmwasm_std::{to_json_binary, Addr, Coin, Decimal256, IbcMsg, IbcTimeout, Order, Storage};
use cw_storage_plus::{Bound, Item, Map, PrefixBound};
use shared::{ibc::TIMEOUT_SECONDS, msg::{contract::{payment::Refund, warehouse::{event::{AddProductEvent, PurchaseEvent}, GroupId, GroupInfo, NewProduct}}, product::{self, Product, ProductId}, purchase::{Purchase, PurchaseId}}};
use anyhow::{Context, Result};

use super::{ibc::IbcChannelKind, State, StateContext};

const GROUP_ID: Item<GroupId> = Item::new("group-id");
const PENDING_PRODUCT_GROUP: Map<ProductId, GroupId> = Map::new("pending-product-group");
const GROUP_TO_PRODUCT: Map<GroupId, ProductId> = Map::new("group-to-product");
const GROUP_OWNER: Map<GroupId, Addr> = Map::new("group-owner");
const HAS_SHIPPED: Map<GroupId, bool> = Map::new("group-has-shipped");
const GROUP_PURCHASES: Map<(GroupId, PurchaseId), ()> = Map::new("group-purchases");
const GROUP_OWNER_LIST: Map<(Addr, GroupId), ()> = Map::new("group-owner-list");
const GROUP_LEN: Map<GroupId, u32> = Map::new("group-len"); 

impl State<'_> {
    pub fn assert_group_not_shipped(&self, store: &dyn Storage, group_id: GroupId) -> Result<()> {
        if HAS_SHIPPED.may_load(store, group_id)?.unwrap_or_default() {
            anyhow::bail!("group {} has already shipped", group_id);
        }
        Ok(())
    }

    pub fn ship_group(&self, ctx: &mut StateContext, msg_sender: Addr, group_id: GroupId) -> Result<()> {
        self.assert_group_not_shipped(ctx.store, group_id)?;

        let group_owner = GROUP_OWNER.load(ctx.store, group_id)?;
        if msg_sender != group_owner {
            anyhow::bail!("only the group owner can ship the group");
        }

        let group_info = self.get_group_info(ctx.store, group_id)?;

        let mut refunds = Vec::new();
        for purchase_id in GROUP_PURCHASES.prefix(group_id).keys(ctx.store, None, None, Order::Ascending) {
            let purchase_id = purchase_id?;
            let purchase = self.try_get_purchase(ctx.store, purchase_id)?.context(format!("purchase not found for group {}", group_id))?;
            let amount_spent = Decimal256::from_ratio(purchase.quantity, 1u32) * group_info.product.price;
            let amount_shipped = Decimal256::from_ratio(purchase.quantity, 1u32) * group_info.cost_per_item();
            let refund = amount_spent - amount_shipped;
            refunds.push(Refund {
                recipient: purchase.spender,
                amount: refund.to_uint_ceil().to_string().parse()?,
            });
        }
        PENDING_PRODUCT_GROUP.remove(ctx.store, group_info.product.id);

        let msg = shared::msg::contract::payment::IbcExecuteMsg::Refund { refunds };

        // outbound IBC message, where packet is then received on other chain
        let channel_id = self
            .get_ibc_channel(ctx.store, IbcChannelKind::Payment)?
            .endpoint
            .channel_id;

        ctx.response_mut().add_message(IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&msg)?,
            timeout: IbcTimeout::with_timestamp(self.env.block.time.plus_seconds(TIMEOUT_SECONDS)),
        });

        HAS_SHIPPED.save(ctx.store, group_id, &true)?;

        Ok(())
    }
    pub fn add_purchase_to_group(&self, ctx: &mut StateContext, purchase_id: PurchaseId, product_id: ProductId) -> Result<GroupId> {
        let group_owner = self.get_product_owner(ctx.store, product_id)?;

        let group_id = match PENDING_PRODUCT_GROUP.may_load(ctx.store, product_id)? {
            Some(group_id) => {
                group_id
            },
            None => {
                let group_id = GROUP_ID.may_load(ctx.store)?.unwrap_or_default();
                GROUP_ID.save(ctx.store, &(group_id + 1))?;
                PENDING_PRODUCT_GROUP.save(ctx.store, product_id, &group_id)?;
                GROUP_TO_PRODUCT.save(ctx.store, group_id, &product_id)?;
                GROUP_OWNER.save(ctx.store, group_id, &group_owner)?;
                group_id
            }
        };

        GROUP_PURCHASES.save(ctx.store, (group_id, purchase_id), &())?;
        GROUP_OWNER_LIST.save(ctx.store, (group_owner, group_id), &())?;
        GROUP_LEN.update(ctx.store, group_id, |x| anyhow::Ok(x.unwrap_or_default() + 1))?;

        Ok(group_id)
    }

    pub fn list_groups(&self, store: &dyn Storage, owner: Option<String>, limit: Option<u32>, start_after: Option<GroupId>) -> Result<Vec<GroupInfo>> {
        let groups = match owner {
            Some(owner) => {
                GROUP_OWNER_LIST
                    .prefix(Addr::unchecked(owner))
                    .keys(store, start_after.map(|start_after| Bound::exclusive(start_after)), None, cosmwasm_std::Order::Ascending)
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|group_id| {
                        let group_id = group_id?;
                        self.get_group_info(store, group_id)
                    })
                    .collect::<Result<Vec<GroupInfo>, _>>()?
            },
            None => {
                GROUP_OWNER
                    .keys(store, start_after.map(|start_after| Bound::exclusive(start_after)), None, cosmwasm_std::Order::Ascending)
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|group_id| {
                        let group_id = group_id?;
                        self.get_group_info(store, group_id)
                    })
                    .collect::<Result<Vec<GroupInfo>, _>>()?
            }
        };
        Ok(groups)
    }

    pub fn get_group_info(&self, store: &dyn Storage, group_id: GroupId) -> Result<GroupInfo> {
        let count = GROUP_LEN.may_load(store, group_id)?.unwrap_or_default();
        let product_id = GROUP_TO_PRODUCT.load(store, group_id)?;
        let product = self.get_product(store, product_id)?;
        let has_shipped = HAS_SHIPPED.may_load(store, group_id)?.unwrap_or_default();
        Ok(GroupInfo {
            id: group_id,
            count,
            product,
            has_shipped
        })
    }

    pub fn remove_purchase_from_group(&self, ctx: &mut StateContext, purchase_id: PurchaseId, product_id: ProductId) -> Result<()> {
        let purchase = self.try_get_purchase(ctx.store, purchase_id)?.context("purchase not found")?;
        self.assert_group_not_shipped(ctx.store, purchase.group_id)?;

        // sanity check
        if PENDING_PRODUCT_GROUP.load(ctx.store, product_id)? != purchase.group_id {
            anyhow::bail!("purchase {} is not in pending group {}", purchase_id, purchase.group_id);
        }

        let len = GROUP_LEN.update(ctx.store, purchase.group_id, |x| anyhow::Ok(x.unwrap_or_default() - 1))?;
        GROUP_PURCHASES.remove(ctx.store, (purchase.group_id, purchase_id));

        // slight optimization - if this was the last item in the group, remove the group
        if len == 0 {
            PENDING_PRODUCT_GROUP.remove(ctx.store, product_id);
            GROUP_TO_PRODUCT.remove(ctx.store, purchase.group_id);
            GROUP_OWNER.remove(ctx.store, purchase.group_id);
        }

        Ok(())
    }

    pub fn get_group_count(&self, store: &dyn Storage, group_id: GroupId) -> Result<u32> {
        Ok(GROUP_LEN.may_load(store, group_id)?.unwrap_or_default())
    }
}