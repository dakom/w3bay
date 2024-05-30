use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{IbcChannel, Uint128};

use crate::msg::{product::ProductId, purchase::{Purchase, PurchaseId}};

#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a purchase order, sent over IBC
    Purchase {
        // The owner address, on the *Nft* chain (not necessarily the sender, nor the warehouse encoding)
        owner: String,
        product_id: ProductId,
        quantity: u32
    }
}

#[cw_serde]
pub enum IbcExecuteMsg {
    Refund {
        refunds: Vec<Refund>
    }
}

#[cw_serde]
pub struct Refund {
    pub recipient: String,
    pub amount: Uint128
}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get general information about the contract 
    #[returns(InfoResp)]
    Info { }
}


#[cw_serde]
pub struct InfoResp {
    pub ibc_channel: Option<IbcChannel>
}