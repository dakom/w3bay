use products::ProductsPage;

use crate::{atoms::{buttons::Squareish1Button, sidebar::Sidebar}, config::CONFIG, prelude::*, route::Route};

mod products;

pub struct MerchantPage {
}

#[derive(Debug, Clone, PartialEq)]
pub enum MerchantSection {
    Products,
    Shipments,
    Sales
}

impl MerchantPage {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        static CONTAINER:Lazy<String> = Lazy::new(|| {
            class! {
                .style("display", "grid")
                .style("grid-template-columns", "auto 1fr")
            }
        });
        html!("div", {
            .class(&*CONTAINER)
            .child(Sidebar::new([
                ("<-- Back", Route::Landing),
                ("Products", Route::Merchant(MerchantSection::Products)),
                ("Shipments", Route::Merchant(MerchantSection::Shipments)),
                ("Sales", Route::Merchant(MerchantSection::Sales)),
            ]).render())
            .child(html!("div", {
                .child(html!("div", {
                    .style("margin-left", "2rem")
                    .class(&*TEXT_SIZE_XLG)
                    .text("Merchant")
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