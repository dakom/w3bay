use futures::StreamExt;
use gloo_timers::future::IntervalStream;
use products::ProductsPage;

use crate::{atoms::{balance::Balance, buttons::Squareish1Button, sidebar::Sidebar}, config::CONFIG, prelude::*, route::Route};

mod products;

pub struct MerchantPage {
}

#[derive(Debug, Clone, PartialEq)]
pub enum MerchantSection {
    Products,
    Shipments,
}

impl MerchantPage {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let balance = Mutable::new(0.0);
        static CONTAINER:Lazy<String> = Lazy::new(|| {
            class! {
                .style("display", "grid")
                .style("grid-template-columns", "auto 1fr")
            }
        });
        html!("div", {
            .class(&*CONTAINER)
            .child(Sidebar::new([
                ("<-- Back", Some(Route::Landing)),
                ("Products", Some(Route::Merchant(MerchantSection::Products))),
                ("Shipments", Some(Route::Merchant(MerchantSection::Shipments))),
            ]).render())
            .child(html!("div", {
                .child(html!("div", {
                    .style("margin-left", "2rem")
                    .child(Balance::new("Merchant".to_string()).render())
                }))
                .child(html!("div", {
                    .style("padding", "2rem")
                    .child_signal(Route::signal().map(|route| {
                        match route {
                            Route::Merchant(MerchantSection::Products) => Some(ProductsPage::new().render()),
                            _ => None
                        }
                    }))
                }))
            }))
        })
    }
}