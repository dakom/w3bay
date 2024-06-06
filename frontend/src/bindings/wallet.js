export function ffi_connect(networkConfig, chainEnv, onConnected, onError) {

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
            return {signer: client.signer, address, client, ...config}
        }
    }

    const connectQuerier = async (config) => {
        const client = await window.CosmWasmJS.createBatchQueryClient(config.rpc_url);
        return {client, ...config};
    }

    if (!window.keplr) {
        alert("Please install keplr extension");
    } else {
        (async () => {
            try {
                const neutron = await connectSigning(chainEnv === "local" ? networkConfig.neutron_local : networkConfig.neutron_testnet);
                const kujira = await connectSigning(chainEnv === "local" ? networkConfig.kujira_local : networkConfig.kujira_testnet);
                const stargaze = await connectSigning(chainEnv === "local" ? networkConfig.stargaze_local : networkConfig.stargaze_testnet);

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

export async function ffi_install_keplr(networkConfig, chainEnv) {
    if (!window.keplr) {
        alert("Please install keplr extension");
        return;
    }

    async function installKeplr(config) {
        const currency = {
            coinDenom: config.denom,
            coinMinimalDenom: config.denom,
            coinDecimals: 6,
            coinGeckoId: config.full_denom,
        }

        const keplrConfig = {
            chainId:  config.chain_id,
            chainName: config.chain_id,
            rpc: config.rpc_url,
            rest: config.rest_url, 
            bip44: {
                coinType: 118,
            },
            bech32Config: {
                bech32PrefixAccAddr: config.addr_prefix,
                bech32PrefixAccPub: `${config.addr_prefix}pub`,
                bech32PrefixValAddr: `${config.addr_prefix}valoper`,
                bech32PrefixValPub: `${config.addr_prefix}valoperpub`,
                bech32PrefixConsAddr: `${config.addr_prefix}valcons`,
                bech32PrefixConsPub: `${config.addr_prefix}valconspub`
            },
            currencies: [currency],
            feeCurrencies: [currency],
            stakeCurrency: currency,
        }

        await window.keplr.experimentalSuggestChain(keplrConfig)
    }

    await installKeplr(chainEnv === "local" ? networkConfig.neutron_local : networkConfig.neutron_testnet);
    await installKeplr(chainEnv === "local" ? networkConfig.kujira_local : networkConfig.kujira_testnet);
    await installKeplr(chainEnv === "local" ? networkConfig.stargaze_local : networkConfig.stargaze_testnet);
}

export async function ffi_contract_query(wallet, contractAddress, msg) {
    return await wallet.client.queryContractSmart(contractAddress, msg);
}

export async function ffi_contract_exec(wallet, contractAddress, msg) {
    const resp = await wallet.client.execute(wallet.address, contractAddress, msg, "auto", "");
    console.log("executed contract", resp);
    return resp;
} 

export async function ffi_contract_exec_funds(wallet, contractAddress, msg, funds) {
    try {
        const resp = await wallet.client.execute(wallet.address, contractAddress, msg, "auto", "", funds);
        console.log("executed contract", resp);
        return resp;
    } catch(e) {
        console.error(e);
    }
} 

export async function ffi_wallet_balance(wallet) {
    const coin = await wallet.client.getBalance(wallet.address, wallet.denom);
    return Number(coin.amount)
}