mod products;
mod purchases;

use products::ProductsPage;
use purchases::PurchasesPage;

use crate::{atoms::{buttons::Squareish1Button, sidebar::Sidebar}, config::CONFIG, prelude::*, route::Route};

pub struct ConsumerPage {
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsumerSection {
    Products,
    Purchases
}

impl ConsumerPage {
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
                ("Products", Route::Consumer(ConsumerSection::Products)),
                ("Purchases", Route::Consumer(ConsumerSection::Purchases)),
            ]).render())
            .child(html!("div", {
                .child(html!("div", {
                    .style("margin-left", "2rem")
                    .class(&*TEXT_SIZE_XLG)
                    .text("Consumer")
                }))
                .child(html!("div", {
                    .style("padding", "2rem")
                    .child_signal(Route::signal().map(|route| {
                        match route {
                            Route::Consumer(ConsumerSection::Products) => Some(ProductsPage::new().render()),
                            Route::Consumer(ConsumerSection::Purchases) => Some(PurchasesPage::new().render()),
                            _ => None
                        }
                    }))
                }))
            }))
        })
    }
}