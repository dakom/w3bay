use awsm_web::env::env_var;
use cosmwasm_std::Addr;
use once_cell::sync::Lazy;

use crate::{page::{consumer::ConsumerSection, merchant::MerchantSection}, prelude::*, route::Route};

#[derive(Debug)]
pub struct Config {
    // the part of the url that is not the domain
    // e.g. in http://example.com/foo/bar, this would be "foo" if we want
    // all parsing to start from /bar
    // it's helpful in shared hosting environments where the app is not at the root
    pub root_path: &'static str,
    pub media_root: &'static str,
    pub default_lang: Option<&'static str>,
    // for debugging, auto connect to wallet
    pub auto_connect: bool,
    // for debugging, jump into an initial page (will wait until wallet is connected, works with auto_connect)
    pub start_route: Mutex<Option<Route>>,
    pub query_poll_delay_ms: u32,
}

impl Config {
    pub fn app_image_url(&self, path: &str) -> String {
        format!("{}/{}", self.media_root, path)
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "dev")] {
        pub static CONFIG: Lazy<Config> = Lazy::new(|| {
            Config {
                root_path: "",
                media_root: "http://localhost:9000",
                default_lang: None,
                auto_connect: true,
                //start_route: Mutex::new(Some(Route::Consumer(ConsumerSection::Purchases))),
                start_route: Mutex::new(None),
                query_poll_delay_ms: 3000,
            }
        });
    } else {
        pub static CONFIG: Lazy<Config> = Lazy::new(|| {
            Config {
                root_path: "w3bay",
                media_root: "/media",
                default_lang: None,
                auto_connect: false,
                start_route: Mutex::new(None),
                query_poll_delay_ms: 3000,

            }
        });
    }
}

/****** Deploy Config ********/

pub const DEPLOY_CONFIG: Lazy<DeployConfig> = Lazy::new(|| {
    let s = include_str!("../../deploy.json");
    serde_json::from_str(s).expect_ext("failed to parse deploy.json")
});

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeployConfig {
    pub warehouse: DeployContactConfig,
    pub payment: DeployContactConfig,
    pub nft: DeployContactConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeployContactConfig{
    #[serde(rename = "codeId")]
    pub code_id: u64,
    pub address: String,
    pub hash: String,
    #[serde(rename = "ibcPort")]
    pub ibc_port: String,
}

/****** Network Config ********/

pub const NETWORK_CONFIG: Lazy<NetworkConfig> = Lazy::new(|| {
    let s = include_str!("../../network.json");
    serde_json::from_str(s).expect_ext("failed to parse network.json")
});

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkConfig {
    pub neutron: NetworkChainConfig,
    pub kujira: NetworkChainConfig,
    pub stargaze: NetworkChainConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkChainConfig {
    pub rpc_url: String,
    pub rest_url: String,
    pub gas_price: String,
    pub full_denom: String,
    pub denom: String,
    pub chain_id: String,
    pub addr_prefix: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ContractName {
    Warehouse,
    Payment,
    Nft
}

impl ContractName {
    pub fn addr(&self) -> Addr {
        match self {
            ContractName::Warehouse => Addr::unchecked(&DEPLOY_CONFIG.warehouse.address),
            ContractName::Payment => Addr::unchecked(&DEPLOY_CONFIG.payment.address),
            ContractName::Nft => Addr::unchecked(&DEPLOY_CONFIG.nft.address),
        }
    }
}

#[allow(dead_code)]
fn get_env(name: &str) -> Option<String> {
    match env_var(name) {
        Ok(value) => {
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        }
        Err(_) => None,
    }
}
