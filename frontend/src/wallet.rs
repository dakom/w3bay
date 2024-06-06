// Wallet is a global static
// it's driven by bindings to cosmosjs
// there's a fair amount of ceremony here to handle all the FFI shenanigans
// but ultimately you get back Rust-friendly wrappers and can ignore the JS side

use cosmwasm_std::{Coin, Event};
use futures::{channel::oneshot, select, FutureExt, };
use futures_signals::signal::{Mutable, Signal};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use shared::tx::CosmosResponseExt;
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use std::future::IntoFuture;
use awsm_web::prelude::*;
use anyhow::{anyhow, Result};
use crate::config::{ContractName, CHAINENV, DEPLOY_CONFIG, NETWORK_CONFIG};


thread_local! {
    static WALLET: Wallet = {
        Wallet::new()
    };
}
pub struct Wallet
{
    instance: Mutable<Option<WalletInstance>>,
}

impl Wallet
{
    fn new() -> Self {
        Self {
            instance: Mutable::new(None),
        }
    }

    // static function, will mutably update the global static
    // which is itself gated in the UI via connected_signal()
    // will return true on success, false on failure
    pub async fn connect() -> bool {

        // there might be a better way to do all this...
        // but bottom line, all the funky ffi stuff is tucked away
        // from an API perspective, we just call an async fn and check the result for error
        
        let (tx_success, rx_success) = oneshot::channel::<WalletInstance>();
        let (tx_fail, rx_fail) = oneshot::channel::<()>();

        let on_connect_signing = Closure::once(move |instance| {
            tx_success.send(instance).unwrap_ext();
        });

        let on_error_signing = Closure::<dyn FnOnce() + 'static>::once(|| {
            tx_fail.send(()).unwrap_ext();
        });
    
        ffi_connect(
            serde_wasm_bindgen::to_value(&*NETWORK_CONFIG).unwrap_ext(),
            CHAINENV.as_str(),
            on_connect_signing.as_ref().unchecked_ref(),
            on_error_signing.as_ref().unchecked_ref(),
        );

        enum WalletResult {
            Success(WalletInstance),
            Failure
        }
        let mut rx_success_fut = rx_success.into_future().map(|instance| {
            match instance {
                Ok(instance) => WalletResult::Success(instance),
                _ => WalletResult::Failure
            }
        });
        let mut rx_fail_fut = rx_fail.into_future().map(|_| {
            WalletResult::Failure
        });

        let res = select! {
            res = rx_success_fut => res, 
            res = rx_fail_fut => res
        };

        match res {
            WalletResult::Success(instance) => {
                WALLET.with(|wallet| {
                    wallet.instance.set(Some(instance));
                });

                true
            },
            WalletResult::Failure => false
        }
    }

    pub async fn install_keplr() -> Result<(), JsValue> {
        log::info!("installing keplr...");
        ffi_install_keplr(
            serde_wasm_bindgen::to_value(&*NETWORK_CONFIG).unwrap_ext(),
            CHAINENV.as_str()
        ).await.map(|_| ())
    }


    pub fn get_connected() -> bool {
        WALLET.with(|wallet| {
            wallet.instance.lock_ref().is_some()
        })
    }
    pub fn connected_signal() -> impl Signal<Item = bool> {
        WALLET.with(|wallet| {
            wallet.instance.signal_ref(|instance| instance.is_some())
        })
    }

    pub fn neutron() -> WalletSigning {
        WALLET.with(|wallet| {
            wallet.instance.lock_ref().as_ref().unwrap_ext().neutron()
        })
    }

    pub fn kujira() -> WalletSigning{
        WALLET.with(|wallet| {
            wallet.instance.lock_ref().as_ref().unwrap_ext().kujira()
        })
    }

    pub fn stargaze() -> WalletSigning {
        WALLET.with(|wallet| {
            wallet.instance.lock_ref().as_ref().unwrap_ext().stargaze()
        })
    }

}

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    pub type WalletInstance;

    #[wasm_bindgen(method, getter)]
    pub fn neutron(this: &WalletInstance) -> WalletSigning;

    #[wasm_bindgen(method, getter)]
    pub fn kujira(this: &WalletInstance) -> WalletSigning;

    #[wasm_bindgen(method, getter)]
    pub fn stargaze(this: &WalletInstance) -> WalletSigning;
}

#[wasm_bindgen]
extern "C" {
    #[derive(Debug, Clone)]
    pub type WalletSigning;

    #[wasm_bindgen(method, getter, js_name = "chainId")]
    pub fn chain_id_string(this: &WalletSigning) -> String;

    #[wasm_bindgen(method, getter)]
    pub fn signer(this: &WalletSigning) -> JsValue;

    #[wasm_bindgen(method, getter)]
    pub fn client(this: &WalletSigning) -> JsValue;

    #[wasm_bindgen(method, getter)]
    pub fn address(this: &WalletSigning) -> String;

    #[wasm_bindgen(method, getter)]
    pub fn denom(this: &WalletSigning) -> String;
}

impl WalletSigning {
    pub fn chain_id(&self) -> ChainId {
        self.chain_id_string().parse().unwrap_ext()
    }

    pub async fn balance(&self) -> Result<f64> {
        ffi_wallet_balance(&self)
            .await
            .and_then(|resp| resp.as_f64().ok_or("balance not a number".into()))
            .map_err(|err| anyhow!("{:?}", err)) 
    }

    pub async fn contract_query<MSG: Serialize, RESP: DeserializeOwned>(
        &self,
        name: ContractName,
        msg: &MSG,
    ) -> Result<RESP> {
        json_deserialize_result(ffi_contract_query(self, name.addr().as_str(), json_serialize(msg)?).await)
    }
    
    pub async fn contract_exec<MSG: Serialize>(
        &self,
        name: ContractName,
        msg: &MSG,
    ) -> Result<TxResp> {
        json_deserialize_result(ffi_contract_exec(self, name.addr().as_str(), json_serialize(msg)?).await)
    }
    
    pub async fn contract_exec_funds<MSG: Serialize>(
        &self,
        name: ContractName,
        msg: &MSG,
        funds: &[Coin],
    ) -> Result<TxResp> {
        let funds = json_serialize(funds)?;
        let resp = json_deserialize_result(
            ffi_contract_exec_funds(self, name.addr().as_str(), json_serialize(msg)?, funds).await,
        )?;
    
        Ok(resp)
    
    }
}

#[wasm_bindgen(module = "/src/bindings/wallet.js")]
extern "C" {
    fn ffi_connect(
        network_config: JsValue,
        chainenv: &str,
        on_connected: &js_sys::Function,
        on_error: &js_sys::Function,
    );

    #[wasm_bindgen(catch)]
    async fn ffi_install_keplr(network_config: JsValue, chainenv: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn ffi_contract_query(
        wallet: &JsValue,
        addr: &str,
        msg: JsValue,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn ffi_contract_exec(
        wallet: &JsValue,
        addr: &str,
        msg: JsValue,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn ffi_contract_exec_funds(
        wallet: &JsValue,
        addr: &str,
        msg: JsValue,
        funds: JsValue,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn ffi_wallet_balance(
        wallet: &JsValue,
    ) -> Result<JsValue, JsValue>;
}

pub enum ChainId {
    Neutron,
    Stargaze,
    Kujira
}

impl std::str::FromStr for ChainId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "neutron" => Ok(ChainId::Neutron),
            "stargaze" => Ok(ChainId::Stargaze),
            "kujira" => Ok(ChainId::Kujira),
            _ => Err(anyhow!("Unknown chain id: {}", s)),
        }
    }
}

// generic helpers for any serializable msg/response
fn json_serialize(data: impl Serialize) -> Result<JsValue> {
    data.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|err| anyhow!("{}", err))
}
fn json_deserialize<T: DeserializeOwned>(data: JsValue) -> Result<T> {
    serde_wasm_bindgen::from_value(data).map_err(|err| anyhow!("{}", err))
}
fn json_deserialize_result<T: DeserializeOwned>(result: Result<JsValue, JsValue>) -> Result<T> {
    match result {
        Ok(data) => json_deserialize(data),
        Err(err) => Err(anyhow!("{:?}", err)),
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TxResp {
    #[serde(rename = "gasUsed")]
    pub gas_used: u64,
    #[serde(rename = "gasWanted")]
    pub gas_wanted: u64,
    #[serde(rename = "height")]
    pub height: u64,
    #[serde(rename = "transactionHash")]
    pub hash: String,
    // will always be 1 deep since we only send one message at a time
    pub logs: Vec<Logs>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Logs {
    pub msg_index: Option<u32>,
    pub log: Option<String>,
    pub events: Vec<cosmwasm_std::Event>,
}

impl CosmosResponseExt for TxResp {
    fn events(&self) -> Box<dyn Iterator<Item = Event> + 'static> {
        Box::new(
            self.logs
                .clone()
                .into_iter()
                .map(|log| log.events.into_iter())
                .flatten(),
        )
    }
}
