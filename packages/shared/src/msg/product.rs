use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal256;

#[cw_serde]
pub struct Product {
    pub id: ProductId,
    pub name: String,
    // max price per item, will be reduced for each person in the OrderGroup
    pub price: Decimal256,
    pub stock: u32
}

pub type ProductId = u32;
