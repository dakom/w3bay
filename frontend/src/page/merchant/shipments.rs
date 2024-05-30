use awsm_web::window;
use dominator_helpers::futures::AsyncLoader;
use shared::{msg::{contract::warehouse::{event::AddProductEvent, GroupId, GroupInfo, NewProduct, QueryMsg}, product::{Product, ProductId}}, tx::CosmosResponseExt};
use wasm_bindgen_futures::spawn_local;

use crate::{atoms::{buttons::{OutlineButton, Squareish1Button}, input::{TextInput, TextInputKind}}, config::ContractName, prelude::*};

pub struct ShipmentsPage {
    pub list: Arc<ListGroups>,
}

impl ShipmentsPage {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            list: ListGroups::new(),
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

struct ListGroups {
    list: MutableVec<GroupInfo>,
    loader: AsyncLoader,
}

impl ListGroups {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            list: MutableVec::new(),
            loader: AsyncLoader::new(),
        })
    }

    fn reload(self: &Arc<Self>) {
        let state = self;
        state.loader.load(clone!(state => async move {
            match load_groups().await {
                Ok(groups) => {
                    state.list.lock_mut().replace_cloned(groups);
                },
                Err(err) => {
                    web_sys::window().unwrap_ext().alert_with_message(&format!("Error loading groups: {:?}", err)).unwrap_throw();
                }
            }
        }))
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
                state.reload()
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
                .children_signal_vec(state.list.signal_vec_cloned().map(clone!(state => move |group| {
                    GroupCard::new(group, state.clone()).render()
                })))
            }))
        })
    }
}

struct GroupCard {
    group: GroupInfo,
    list: Arc<ListGroups>
}

impl GroupCard {
    fn new(group: GroupInfo, list: Arc<ListGroups>) -> Arc<Self> {
        Arc::new(Self {
            group,
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
                    .text(&self.group.product.name)
                }),
                html!("div", {
                    .text(&format!("Members: {}", self.group.count))
                }),
                html!("div", {
                    .text(&format!("Has shipped: {}", self.group.has_shipped))
                }),
                OutlineButton::new(true).render(None, "Ship".to_string(), clone!(state => move || {
                    spawn_local(clone!(state => async move {
                        Wallet::neutron().contract_exec(ContractName::Warehouse, &WarehouseExecuteMsg::ShipGroup {
                            group_id: state.group.id.clone(),
                        }).await;
                        state.list.reload();
                    }))
                }))
            ])
        })
    }
}

async fn load_groups() -> Result<Vec<GroupInfo>> {
    let mut start_after: Option<GroupId> = None;
    let mut groups:Vec<GroupInfo> = vec![];
    loop {
        let res:Result<Vec<GroupInfo>> = Wallet::neutron().contract_query(ContractName::Warehouse, &WarehouseQueryMsg::ListGroups {
            owner: Some(Wallet::neutron().address().to_string()),
            limit: None,
            start_after,
        }).await;

        match res {
            Ok(res) => match res.last().cloned() {
                Some(last) => {
                    groups.extend(res);
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

    Ok(groups)
}