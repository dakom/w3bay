use awsm_web::window;
use dominator_helpers::futures::AsyncLoader;
use shared::{msg::{contract::warehouse::{event::AddProductEvent, NewProduct, QueryMsg}, product::{Product, ProductId}}, tx::CosmosResponseExt};

use crate::{atoms::{buttons::Squareish1Button, input::{TextInput, TextInputKind}}, config::ContractName, prelude::*};

pub struct ProductsPage {
    pub add: Arc<AddProduct>,
    pub list: Arc<ListProducts>,
}

impl ProductsPage {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            add: AddProduct::new(),
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
            .child(state.add.render(clone!(state => move || {
                if let Some(product) = state.add.get_product() {
                    state.add.loader.load(clone!(state => async move {
                        let resp = Wallet::neutron().contract_exec(ContractName::Warehouse, &WarehouseExecuteMsg::AddProduct {
                            product: product.clone(),
                        }).await.unwrap_ext();

                        let evt:AddProductEvent = resp.event_first(AddProductEvent::KEY).map(|evt| evt.try_into()).unwrap_ext().unwrap_ext();

                        state.list.list.lock_mut().push_cloned(evt.product);
                    }));
                } else {
                    web_sys::window().unwrap_ext().alert_with_message("Invalid product data").unwrap_throw();
                }
            })))
            .child(state.list.render())
        })
    }

}

struct AddProduct {
    name: TextInput,
    price: TextInput,
    stock: TextInput,
    loader: AsyncLoader,
}

impl AddProduct {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            name: TextInput::new(TextInputKind::Text),
            price: TextInput::new(TextInputKind::Number),
            stock: TextInput::new(TextInputKind::Number),
            loader: AsyncLoader::new(),
        })
    }

    fn render(self: &Arc<Self>, on_add: impl Fn() + 'static) -> Dom 
    {
        let state = self;
        static CONTAINER:Lazy<String> = Lazy::new(|| {
            class! {
                .style("display", "flex")
                .style("gap", "1rem")
                .style("padding-bottom", "1rem")
            }
        });
        html!("div", {
            .style("border-bottom", "1px solid")
            .child(html!("div", {
                .class(&*CONTAINER)
                .child(state.name.render(Some("Product Name")))
                .child(state.price.render(Some("Price")))
                .child(state.stock.render(Some("Current Stock")))
                .child(Squareish1Button::new().render("Add Product".to_string(), on_add))
            }))
            .child_signal(state.loader.is_loading().map(|loading| {
                if loading {
                    Some(html!("div", {
                        .text("Adding product...")
                    }))
                } else {
                    None
                }
            }))
        })
    }

    fn get_product(&self) -> Option<NewProduct> {
        let name = self.name.value.get_cloned()?;
        let price = self.price.value.get_cloned()?.parse().ok()?;
        let stock = self.stock.value.get_cloned()?.parse().ok()?;
        Some(NewProduct {
            name,
            price,
            stock,
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
    product: Product
}

impl ProductCard {
    fn new(product: Product) -> Arc<Self> {
        Arc::new(Self {
            product,
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
            ])
        })
    }
}

async fn load_products() -> Result<Vec<Product>> {
    let mut start_after: Option<u32> = None;
    let mut products:Vec<Product> = vec![];
    loop {
        let res:Result<Vec<Product>> = Wallet::neutron().contract_query(ContractName::Warehouse, &WarehouseQueryMsg::ListProducts {
            owner: Some(Wallet::neutron().address().to_string()),
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