use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal256, IbcChannel, Uint128};

use crate::msg::{product::{Product, ProductId}, purchase::{Purchase, PurchaseId}};

#[cw_serde]
pub enum ExecuteMsg {
    AddProduct {
        product: NewProduct,
    },
    ShipGroup {
        group_id: GroupId,
    },
}


#[cw_serde]
pub enum IbcExecuteMsg {
    Purchase {
        // The owner address, on the *Nft* chain (not necessarily the spender)
        owner: String,
        // The spender address, on the *Payment* chain (not necessarily the owner)
        spender: String,
        // The product ID to purchase
        product_id: ProductId, 
        // the quantity of products to purchase
        quantity: u32,
        // fees sent
        fees: Uint128 
    },
    RemovePurchase {
        id: PurchaseId
    },
}

#[cw_serde]
pub struct NewProduct {
    pub name: String,
    // max price per item, will be reduced for each person in the OrderGroup
    pub price: Decimal256,
    pub stock: u32
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the current products in the warehouse 
    #[returns(Vec<Product>)]
    ListProducts { 
        owner: Option<String>,
        limit: Option<u32>,
        start_after: Option<ProductId>
    },
    /// Returns the groups that are pending shipment 
    #[returns(Vec<GroupInfo>)]
    ListGroups { 
        owner: Option<String>,
        limit: Option<u32>,
        start_after: Option<GroupId>
    },
    /// Returns the current purchases in the warehouse for a given user
    #[returns(Vec<Purchase>)]
    GetPurchases { 
        ids: Vec<PurchaseId>,
    },
    /// Returns the products for the given ids
    #[returns(Vec<Product>)]
    GetProducts { 
        ids: Vec<ProductId>,
    },
    /// Returns the group infos for the given ids
    #[returns(Vec<GroupInfo>)]
    GetGroups { 
        ids: Vec<GroupId>,
    },
    /// Get general information about the contract 
    #[returns(InfoResp)]
    Info { },
}

#[cw_serde]
pub struct InfoResp {
    pub ibc_payment_channel: Option<IbcChannel>
}

#[cw_serde]
pub struct PurchaseError {
    // The owner address, on the *Warehouse* chain (not necessarily the spender)
    pub owner: String,
    // The spender address, on the *Payment* chain (not necessarily the owner)
    pub spender: String,
    // The product ID to purchase
    pub product_id: ProductId, 
    // the quantity of products to purchase
    pub quantity: u32,
    // fees sent
    pub fees: Uint128,
    // reason
    pub reason: String 
}

impl std::fmt::Display for PurchaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "purchase error: owner: {}, spender: {}, product_id: {}, quantity: {}, funds: {:?}, reason: {}", self.owner, self.spender, self.product_id, self.quantity, self.fees, self.reason)
    }
}
impl std::error::Error for PurchaseError {}

pub type GroupId = u64;

#[cw_serde]
pub struct GroupInfo {
    pub id: GroupId,
    pub count: u32,
    pub product: Product,
    pub has_shipped: bool
}

impl GroupInfo {
    pub fn discount_perc(&self) -> Decimal256 {
        let one = Decimal256::one();
        let count = Decimal256::from_ratio(self.count, 1u32);

        // TEST ME
        (one - (one / (one + count))) * Decimal256::percent(100)
    }

    pub fn cost_per_item(&self) -> Decimal256 {
        self.product.price * (Decimal256::one() - self.discount_perc())
    }
}

pub mod event {
    use cosmwasm_std::{Addr, Event};
    use anyhow::{Error, anyhow};
    use crate::{event::CosmwasmEventExt, msg::{product::Product, purchase::Purchase}};

    /// Event emitted when a new product is added to the warehouse 
    #[derive(Debug)]
    pub struct AddProductEvent {
        pub product: Product,
    }

    impl AddProductEvent {
        pub const KEY: &'static str = "add-product";
    }

    impl From<AddProductEvent> for Event {
        fn from(src: AddProductEvent) -> Self {
            Event::new(AddProductEvent::KEY).add_attributes(vec![
                ("id", src.product.id.to_string()),
                ("name", src.product.name.to_string()),
                ("price", src.product.price.to_string()),
                ("stock", src.product.stock.to_string()),
            ])
        }
    }

    impl TryFrom<Event> for AddProductEvent {
        type Error = Error;

        fn try_from(evt: Event) -> anyhow::Result<Self> {
            if evt.ty.as_str() != format!("wasm-{}", AddProductEvent::KEY) {
                return Err(anyhow!("unexpected event type: {}, should be {}", evt.ty, AddProductEvent::KEY));
            }

            Ok(AddProductEvent{
                product: Product {
                    id: evt.string_attr("id")?.parse()?,
                    name: evt.string_attr("name")?,
                    price: evt.string_attr("price")?.parse()?,
                    stock: evt.string_attr("stock")?.parse()?,
                }
            })
        }
    }


    /// Event emitted when a new purchase is made in the warehouse 
    #[derive(Debug)]
    pub struct PurchaseEvent {
        pub purchase: Purchase,
    }

    impl PurchaseEvent {
        pub const KEY: &'static str = "purchase";
    }

    impl From<PurchaseEvent> for Event {
        fn from(src: PurchaseEvent) -> Self {
            Event::new(PurchaseEvent::KEY).add_attributes(vec![
                ("id", src.purchase.id.to_string()),
                ("product-id", src.purchase.product_id.to_string()),
                ("group-id", src.purchase.group_id.to_string()),
                ("quantity", src.purchase.quantity.to_string()),
                ("spender", src.purchase.spender.to_string()),
            ])
        }
    }

    impl TryFrom<Event> for PurchaseEvent {
        type Error = Error;

        fn try_from(evt: Event) -> anyhow::Result<Self> {
            if evt.ty.as_str() != format!("wasm-{}", PurchaseEvent::KEY) {
                return Err(anyhow!("unexpected event type: {}, should be {}", evt.ty, PurchaseEvent::KEY));
            }

            Ok(PurchaseEvent {
                purchase: Purchase{
                    id: evt.string_attr("id")?.parse()?,
                    product_id: evt.string_attr("product-id")?.parse()?,
                    group_id: evt.string_attr("group-id")?.parse()?,
                    quantity: evt.string_attr("quantity")?.parse()?,
                    spender: evt.string_attr("spender")?.parse()?,
                }
            })
        }
    }
}
