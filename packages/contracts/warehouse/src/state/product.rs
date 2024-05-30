use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::{Bound, Map};
use shared::msg::{contract::warehouse::{event::AddProductEvent, NewProduct}, product::{Product, ProductId}};
use anyhow::Result;

use super::{State, StateContext};

const PRODUCTS: Map<ProductId, NewProduct> = Map::new("products");
const PRODUCT_OWNERS: Map<ProductId, Addr> = Map::new("product-owners");
const PRODUCT_OWNER_LIST: Map<(Addr, ProductId), ()> = Map::new("product-owner-list");

impl State<'_> {
    pub fn add_product(&self, ctx: &mut StateContext, owner: Addr, product: NewProduct) -> Result<Product> {
        let id = PRODUCTS
            .keys(ctx.store, None, None, cosmwasm_std::Order::Descending)
            .next()
            .transpose()?
            .map_or(0, |x| x + 1);

        PRODUCTS.save(ctx.store, id, &product)?;
        PRODUCT_OWNERS.save(ctx.store, id, &owner)?;
        PRODUCT_OWNER_LIST.save(ctx.store, (owner, id), &())?;

        ctx.response.add_event(AddProductEvent {
            product: Product {
                id,
                name: product.name.clone(),
                price: product.price,
                stock: product.stock
            }
        });

        Ok(Product {
            id,
            name: product.name,
            price: product.price,
            stock: product.stock
        })
    }

    pub fn add_product_stock(&self, ctx: &mut StateContext, id: ProductId, quantity: u32) -> Result<()> {
        let mut product = PRODUCTS.load(ctx.store, id)?;
        product.stock += quantity;
        PRODUCTS.save(ctx.store, id, &product)?;

        Ok(())
    }

    pub fn remove_product_stock(&self, ctx: &mut StateContext, id: ProductId, quantity: u32) -> Result<()> {
        let mut product = PRODUCTS.load(ctx.store, id)?;
        if product.stock < quantity {
            anyhow::bail!("not enough stock to remove (wanted: {}, available: {})", quantity, product.stock);
        }
        product.stock -= quantity;
        PRODUCTS.save(ctx.store, id, &product)?;

        Ok(())
    }

    pub fn list_products(&self, store: &dyn Storage, owner: Option<String>, limit: Option<u32>, start_after: Option<ProductId>) -> Result<Vec<Product>> {
        match owner {
            None => {
                let products = PRODUCTS
                    .range(store, start_after.map(|start_after| Bound::exclusive(start_after)), None, cosmwasm_std::Order::Ascending)
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|res| {
                        let (id, product) = res?;
                        anyhow::Ok(Product {
                            id,
                            name: product.name.clone(),
                            price: product.price,
                            stock: product.stock
                        })
                    })
                    .collect::<Result<Vec<Product>, _>>()?;

                Ok(products)
            },
            Some(owner) => {
                let products = PRODUCT_OWNER_LIST
                    .prefix(Addr::unchecked(owner)) 
                    .keys(store, start_after.map(|start_after| Bound::exclusive(start_after)), None, cosmwasm_std::Order::Ascending)
                    .take(limit.unwrap_or(u32::MAX) as usize)
                    .map(|id| {
                        let id = id?;
                        let product = self.get_product(store, id)?;
                        anyhow::Ok(Product {
                            id,
                            name: product.name.clone(),
                            price: product.price,
                            stock: product.stock
                        })
                    })
                    .collect::<Result<Vec<Product>, _>>()?;

                Ok(products)
            }
        }
    }

    pub fn get_product(&self, store: &dyn Storage, id: ProductId) -> Result<Product> {
        PRODUCTS
            .load(store, id)
            .map(|product| Product {
                id,
                name: product.name,
                price: product.price,
                stock: product.stock
            })
            .map_err(|err| err.into())
    }

    pub fn get_product_owner(&self, store: &dyn Storage, id: ProductId) -> Result<Addr> {
        PRODUCT_OWNERS
            .load(store, id)
            .map_err(|err| err.into())
    }
    pub fn get_products(&self, store: &dyn Storage, ids: Vec<ProductId>) -> Result<Vec<Product>> {
        ids
            .into_iter()
            .map(|id| {
                PRODUCTS
                    .load(store, id)
                    .map(|product| Product {
                        id,
                        name: product.name,
                        price: product.price,
                        stock: product.stock
                    })
                    .map_err(|err| err.into())
            })
            .collect::<Result<Vec<Product>, _>>()
    }
}