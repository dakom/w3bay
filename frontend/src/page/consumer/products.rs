use awsm_web::window;
use cosmwasm_std::Coin;
use dominator_helpers::futures::AsyncLoader;
use shared::{msg::{contract::{payment::InfoResp, warehouse::{event::AddProductEvent, NewProduct, QueryMsg}}, product::{Product, ProductId}}, tx::CosmosResponseExt};

use crate::{atoms::{buttons::Squareish1Button, input::{TextInput, TextInputKind}}, config::{ContractName, NETWORK_CONFIG}, prelude::*};

pub struct ProductsPage {
    pub list: Arc<ListProducts>,
}

impl ProductsPage {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            list: ListProducts::new(),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;

        static CONTAINER:Lazy<String> = Lazy::new(|| {
            class! {
            }
        });

        html!("div", {
            .class(&*CONTAINER)
            .class(&*TEXT_SIZE_LG)
            .child(state.list.render())
        })
    }

}

struct ListProducts {
    list: MutableVec<Product>,
    loader: AsyncLoader,
}

impl ListProducts {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            list: MutableVec::new(),
            loader: AsyncLoader::new(),
        })
    }

    fn render(self: &Arc<Self>) -> Dom {
        let state = self;
        static CONTAINER:Lazy<String> = Lazy::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("gap", "1rem")
            }
        });
        html!("div", {
            .after_inserted(clone!(state => move |_| {
                state.loader.load(clone!(state => async move {
                    match load_products().await {
                        Ok(products) => {
                            state.list.lock_mut().replace_cloned(products);
                        },
                        Err(err) => {
                            web_sys::window().unwrap_ext().alert_with_message(&format!("Error loading products: {:?}", err)).unwrap_throw();
                        }
                    }
                }))
            }))
            .class(&*CONTAINER)
            .child_signal(state.loader.is_loading().map(|loading| {
                if loading {
                    Some(html!("div", {
                        .text("Loading...")
                    }))
                } else {
                    None
                }
            }))
            .child(html!("div", {
                .style("margin-top", "1rem")
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("gap", "1rem")
                .children_signal_vec(state.list.signal_vec_cloned().map(|product| {
                    ProductCard::new(product).render()
                }))
            }))
        })
    }
}

struct ProductCard {
    product: Product,
    loader: AsyncLoader
}

impl ProductCard {
    fn new(product: Product) -> Arc<Self> {
        Arc::new(Self {
            product,
            loader: AsyncLoader::new(),
        })
    }

    fn render(self: &Arc<Self>) -> Dom {
        let state = self;
        static CONTAINER:Lazy<String> = Lazy::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("gap", "1rem")
                .style("padding", "1rem")
                .style("border", "1px solid")
            }
        });
        html!("div", {
            .class(&*CONTAINER)
            .children(&mut [
                html!("div", {
                    .text(&self.product.name)
                }),
                html!("div", {
                    .text(&format!("Price: {}", self.product.price))
                }),
                html!("div", {
                    .text(&format!("Stock: {}", self.product.stock))
                }),
                html!("div", {
                    .text_signal(state.loader.is_loading().map(|loading| {
                        if loading {
                            "Loading..."
                        } else {
                            ""
                        }
                    }))
                }),
                Squareish1Button::new().render("Buy".to_string(), clone!(state => move || {
                    // Sends the funds to Kujira,
                    // which will then send the actual purchase to Neutron via IBC
                    state.loader.load(clone!(state => async move {
                        let info:InfoResp = Wallet::kujira().contract_query(ContractName::Payment, &PaymentQueryMsg::Info {  }).await.unwrap_ext();
                        log::info!("info: {:?}", info);

                        let quantity = 1u32;
                        let amount = state.product.price.checked_mul(quantity.to_string().parse().unwrap_ext()).unwrap_ext();
                        Wallet::kujira().contract_exec_funds(
                            ContractName::Payment, 
                            &PaymentExecuteMsg::Purchase {
                                owner: Wallet::stargaze().address(),
                                product_id: state.product.id.clone(),
                                quantity,
                            },
                            &[Coin {
                                denom: NETWORK_CONFIG.kujira.denom.clone(),
                                amount: amount.to_uint_ceil().to_string().parse().unwrap_ext()
                            }]
                        ).await.unwrap_throw();
                    }))
                }))
            ])
        })
    }
}

async fn load_products() -> Result<Vec<Product>> {
    let mut start_after: Option<u32> = None;
    let mut products:Vec<Product> = vec![];
    loop {
        let res:Result<Vec<Product>> = Wallet::neutron().contract_query(ContractName::Warehouse, &WarehouseQueryMsg::ListProducts {
            owner: None,
            limit: None,
            start_after,
        }).await;

        match res {
            Ok(res) => match res.last().cloned() {
                Some(last) => {
                    products.extend(res);
                    start_after = Some(last.id);
                }
                None => {
                    break;
                }
            },
            Err(err) => {
                return Err(err);
            }
        }
    }

    Ok(products)
}