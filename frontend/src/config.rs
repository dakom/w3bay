use core::panic;

use awsm_web::env::{self, env_var};
use cosmwasm_std::Addr;
use once_cell::sync::Lazy;

use crate::{page::{consumer::ConsumerSection, merchant::MerchantSection}, prelude::*, route::Route};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Environment {
    Local,
    Testnet,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Testnet => "testnet",
        }
    }
}

impl From<std::option::Option<&str>> for Environment {
    fn from(s: Option<&str>) -> Self {
        match s {
            Some("local") => Environment::Local,
            Some("testnet") => Environment::Testnet,
            _ => panic!("invalid CHAINENV, set env var to 'local' or 'testnet'"),
        }
    }
}

pub static CHAINENV: Lazy<Environment> = Lazy::new(|| option_env!("CHAINENV").into());

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
                //auto_connect: false,
                start_route: Mutex::new(Some(Route::Merchant(MerchantSection::Shipments))),
                //start_route: Mutex::new(None),
                query_poll_delay_ms: 3000,
            }
        });
    } else {
        pub static CONFIG: Lazy<Config> = Lazy::new(|| {
            Config {
                root_path: "w3bay",
                media_root: "https://dakom.github.io/w3bay/media",
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
    pub warehouse_testnet: DeployContactConfig,
    pub payment_testnet: DeployContactConfig,
    pub nft_testnet: DeployContactConfig,
    pub warehouse_local: DeployContactConfig,
    pub payment_local: DeployContactConfig,
    pub nft_local: DeployContactConfig,
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
    pub neutron_testnet: NetworkChainConfig,
    pub kujira_testnet: NetworkChainConfig,
    pub stargaze_testnet: NetworkChainConfig,
    pub neutron_local: NetworkChainConfig,
    pub kujira_local: NetworkChainConfig,
    pub stargaze_local: NetworkChainConfig,
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
    fn deploy_config(&self) -> DeployContactConfig {
        let env = *CHAINENV;
        match self {
            ContractName::Warehouse => match env {
                Environment::Local => DEPLOY_CONFIG.warehouse_local.clone(),
                Environment::Testnet => DEPLOY_CONFIG.warehouse_testnet.clone(),
            },
            ContractName::Payment => match env {
                Environment::Local => DEPLOY_CONFIG.payment_local.clone(),
                Environment::Testnet => DEPLOY_CONFIG.payment_testnet.clone(),
            },
            ContractName::Nft => match env {
                Environment::Local => DEPLOY_CONFIG.nft_local.clone(),
                Environment::Testnet => DEPLOY_CONFIG.nft_testnet.clone(),
            },
        }
    }

    pub fn addr(&self) -> Addr {
        Addr::unchecked(self.deploy_config().address)
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
