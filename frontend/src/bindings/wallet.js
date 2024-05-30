export function ffi_connect(networkConfig, onConnected, onError) {

    const connectSigning = async (config) => {
        const {
            rpc_url,
            chain_id,
            denom,
            addr_prefix,
            gas_price,
            full_denom
        } = config;

        await window.keplr.enable(chain_id);
        const offlineSigner = await window.getOfflineSigner(chain_id);
        const client = await window.CosmWasmJS.SigningCosmWasmClient.connectWithSigner(
            rpc_url,
            offlineSigner,
            { 
                gasPrice: window.CosmWasmJS.GasPrice.fromString(`${gas_price}${denom}`), 
            }
        );

        const accounts = await client.signer.getAccounts();
        if(!accounts || !accounts.length) {
            throw new Error(`gotta get some funds first!`);
        } else {
            const address = accounts[0].address;
            return {signer: client.signer, address, client, config}
        }
    }

    const connectQuerier = async (config) => {
        const client = await window.CosmWasmJS.createBatchQueryClient(config.rpc_url);
        return {client, config};
    }

    if (!window.keplr) {
        alert("Please install keplr extension");
    } else {
        (async () => {
            try {
                const neutron = await connectSigning(networkConfig.neutron);
                const kujira = await connectSigning(networkConfig.kujira);
                const stargaze = await connectSigning(networkConfig.stargaze);

                onConnected({
                    neutron: {
                        ...neutron,
                        chainId: "neutron"
                    },
                    kujira: {
                        ...kujira,
                        chainId: "kujira"
                    },
                    stargaze: {
                        ...stargaze, 
                        chainId: "stargaze",
                    },
                })
            } catch(e) {
                console.error(e);
                onError();
            }
        })();
    }
}

export function ffi_install_keplr(onInstalled) {
    if (!window.keplr) {
        alert("Please install keplr extension");
        return;
    }
    const currency = {
        coinDenom: CURRENT_FAMILY_CONFIG.denom,
        coinMinimalDenom: CURRENT_FAMILY_CONFIG.denom,
        coinDecimals: 6,
        coinGeckoId: CURRENT_FAMILY_CONFIG.fullDenom,
    }

    const keplrConfig = {
        chainId:  CURRENT_FAMILY_CONFIG.chainId,
        chainName: CURRENT_FAMILY_CONFIG.chainId,
        rpc: CURRENT_FAMILY_CONFIG.rpcUrl,
        rest: CURRENT_FAMILY_CONFIG.restUrl, 
        bip44: {
            coinType: 118,
        },
        bech32Config: {
            bech32PrefixAccAddr: CURRENT_FAMILY_CONFIG.addressPrefix,
            bech32PrefixAccPub: `${CURRENT_FAMILY_CONFIG.addressPrefix}pub`,
            bech32PrefixValAddr: `${CURRENT_FAMILY_CONFIG.addressPrefix}valoper`,
            bech32PrefixValPub: `${CURRENT_FAMILY_CONFIG.addressPrefix}valoperpub`,
            bech32PrefixConsAddr: `${CURRENT_FAMILY_CONFIG.addressPrefix}valcons`,
            bech32PrefixConsPub: `${CURRENT_FAMILY_CONFIG.addressPrefix}valconspub`
        },
        currencies: [currency],
        feeCurrencies: [currency],
        stakeCurrency: currency,
    }

    window.keplr.experimentalSuggestChain(keplrConfig)
        .then(onInstalled)
        .catch(console.error);
}

export async function ffi_contract_query(wallet, contractAddress, msg) {
    return await wallet.client.queryContractSmart(contractAddress, msg);
}

export async function ffi_contract_exec(wallet, contractAddress, msg) {
    const resp = await wallet.client.execute(wallet.address, contractAddress, msg, "auto", "");
    return resp;
} 

export async function ffi_contract_exec_funds(wallet, contractAddress, msg, funds) {
    console.log("executing contract with funds", contractAddress, msg, funds);
    try {
        const resp = await wallet.client.execute(wallet.address, contractAddress, msg, "auto", "", funds);
        console.log("executed contract", resp);
        return resp;
    } catch(e) {
        console.error(e);
    }
} 

export async function ffi_wallet_balance(wallet) {
    const coin = await wallet.client.getBalance(wallet.address, wallet.config.denom);
    return Number(coin.amount)
}