use std::{cell::Cell, collections::{HashMap, HashSet}};

use awsm_web::window;
use dominator_helpers::futures::AsyncLoader;
use futures::StreamExt;
use gloo_timers::future::IntervalStream;
use shared::{msg::{contract::{nft::TokensResponse, warehouse::{event::AddProductEvent, GroupId, GroupInfo, NewProduct, QueryMsg}}, product::{Product, ProductId}, purchase::{self, Purchase, PurchaseId}}, tx::CosmosResponseExt};
use wasm_bindgen_futures::spawn_local;

use crate::{atoms::{buttons::{OutlineButton, Squareish1Button}, input::{TextInput, TextInputKind}}, config::{ContractName, CONFIG}, prelude::*};

pub struct PurchasesPage {
    pub list: Arc<ListPurchases>,
}

impl PurchasesPage{
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            list: ListPurchases::new(),
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

struct ListPurchases {
    loader: Arc<PurchaseLoader>,
}

impl ListPurchases {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            loader: PurchaseLoader::new(),
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
            .future(state.loader.clone().start_query_loop())
            .class(&*CONTAINER)
            .child(html!("div", {
                .style("margin-top", "1rem")
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("gap", "1rem")
                .children_signal_vec(state.loader.list.signal_vec_cloned().map(clone!(state => move |purchase| {
                    let group = state.loader.groups.lock().unwrap_ext().get(&purchase.group_id).cloned().unwrap();
                    PurchaseCard::new(purchase, group, state.clone()).render()
                })))
            }))
            .child(html!("div", {
                .text_signal(state.loader.is_loading.signal().map(|loading| {
                    if loading {
                        "Loading..."
                    } else {
                        ""
                    }
                }))
            }))
        })
    }
}

struct PurchaseCard {
    purchase: Purchase,
    group: Arc<GroupInfo>,
    loader: AsyncLoader,
    list: Arc<ListPurchases>,
}

impl PurchaseCard {
    fn new(purchase: Purchase, group: Arc<GroupInfo>, list: Arc<ListPurchases>) -> Arc<Self> {
        Arc::new(Self {
            purchase,
            group,
            loader: AsyncLoader::new(),
            list,
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
                    .children([
                        html!("div", {
                            .text(&format!("{}", self.group.product.name))
                        }),
                        html!("div", {
                            .text(&format!("Original price: {}", self.group.product.price))
                        }),
                        html!("div", {
                            .text(&format!("Current price: {}", self.group.cost_per_item()))
                        }),
                        html!("div", {
                            .text(&format!("Shipping status: {}", match self.group.has_shipped {
                                true => "Shipped",
                                false => "Not shipped"
                            }))
                        }),
                        OutlineButton::new(true).render(None, get_text!("button-remove"), clone!(state => move || {
                            let purchase_id = state.purchase.id;
                            let list = state.list.clone();
                            spawn_local(async move {
                                Wallet::stargaze().contract_exec(ContractName::Nft, &NftExecuteMsg::Burn { 
                                    token_id: purchase_id.to_string()
                                }).await;
                                list.loader.reload();
                            })
                        }))
                    ])
                }),
            ])
        })
    }
}

// this is actually fairly complicated... we want:
//
// 1. to not miss any data as it gets populated behind the scenes via IBC
// 2. to not re-render everything as we get new data
// 3. to not query too often
// 4. load the info from different contracts and chains (stargaze, neutron)
// 5. cancel when dropped
struct PurchaseLoader {
    pub is_loading: Mutable<bool>,
    pub list: MutableVec<Purchase>,
    pub last_load_time: Cell<f64>,
    pub error: Mutable<Option<String>>,
    pub groups: Mutex<HashMap<GroupId, Arc<GroupInfo>>>,
}

impl PurchaseLoader {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            is_loading: Mutable::new(false),
            list: MutableVec::new(),
            last_load_time: Cell::new(0.0),
            error: Mutable::new(None),
            groups: Mutex::new(HashMap::new()),
        })
    }

    pub fn reload(&self) {
        self.list.lock_mut().clear();
        self.groups.lock().unwrap_ext().clear();
    }
    pub async fn start_query_loop(self: Arc<Self>) {
        let state = self;
        // check every 10 ms if we're ready to load
        // but the actual loading is throttled by CONFIG.query_poll_delay_ms
        IntervalStream::new(10).for_each(clone!(state => move |_| {
            clone!(state => async move {
                if !state.is_loading.get() {
                    let curr_time = web_sys::window().unwrap_ext().performance().unwrap_ext().now();
                    let last_time = state.last_load_time.get();
                    let diff_time = curr_time - last_time;
                    if last_time == 0.0 || diff_time > CONFIG.query_poll_delay_ms as f64 {
                        state.is_loading.set_neq(true);
                        state.error.set_neq(None);
                        match state.clone().load().await {
                            Err(err) => {
                                log::error!("Error loading: {:?}", err);
                                state.error.set(Some(format!("{:?}", err)));
                            },
                            Ok(items) => {
                                state.list.lock_mut().extend(items);
                            }
                        }
                        state.last_load_time.set(web_sys::window().unwrap_ext().performance().unwrap_ext().now());
                        state.is_loading.set_neq(false);
                    }
                }
            })
        })).await;
    }

    async fn load(self: Arc<Self>) -> Result<Vec<Purchase>> {
        let state = self;
        let new_token_ids = state.load_new_token_ids().await?;
        let mut list: Vec<Purchase> = Vec::with_capacity(new_token_ids.len());

        let mut new_group_ids: Vec<GroupId> = Vec::new();

        for token_ids in new_token_ids.chunks(100) {


            let res:Vec<Purchase> = Wallet::neutron().contract_query(ContractName::Warehouse, &WarehouseQueryMsg::GetPurchases {
                ids: token_ids.iter().map(|id| id.parse().unwrap()).collect::<Vec<PurchaseId>>()
            }).await?;

            for purchase in res.iter() {
                if state.groups.lock().unwrap_ext().get(&purchase.group_id).is_none() {
                    new_group_ids.push(purchase.group_id.clone());
                }
            }
            list.extend(res);
        }

        let mut new_groups: Vec<GroupInfo> = Wallet::neutron().contract_query(ContractName::Warehouse, &WarehouseQueryMsg::GetGroups { 
            ids: new_group_ids
        }).await?;

        let mut lock = state.groups.lock().unwrap_ext();
        for group in new_groups.into_iter() {
            log::info!("group: {:?}", group);
            lock.insert(group.id, Arc::new(group));
        }

        Ok(list)
    }

    async fn load_new_token_ids(&self) -> Result<Vec<String>> {
        let mut token_ids: Vec<String> = vec![];

        let mut start_after: Option<String> = None;
        let owner = Wallet::stargaze().address(); 
        loop {
            let res:Result<TokensResponse> = Wallet::stargaze().contract_query(ContractName::Nft, &NftQueryMsg::Tokens {
                owner: owner.clone(),
                start_after,
                limit: None,
            }).await;

            match res {
                Ok(res) => match res.tokens.last().cloned() {
                    Some(last) => {
                        token_ids.extend(res.tokens);
                        start_after = Some(last);
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

        let mut lock = self.list.lock_mut();
        token_ids.retain(|token_id| lock.iter().find(|item| item.id.to_string() == *token_id).is_none());

        Ok(token_ids)
    }
}
