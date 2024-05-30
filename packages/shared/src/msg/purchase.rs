use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal256};

use super::{contract::warehouse::GroupId, product::ProductId};

#[cw_serde]
pub struct Purchase {
    pub id: PurchaseId,
    pub product_id: ProductId,
    pub quantity: u32,
    pub spender: String,
    pub group_id: GroupId
}

pub type PurchaseId = u64;


