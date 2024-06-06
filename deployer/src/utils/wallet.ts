import * as fs from "fs-extra"; 
import { ExecuteInstruction, ExecuteResult, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate"
import { 
    Account,
    GasPrice,
    StdFee
} from "@cosmjs/stargate"
import { Coin, DirectSecp256k1HdWallet, Registry } from "@cosmjs/proto-signing"
import { ContractName, Environment, NetworkConfig, Target, getContractPath, getDeployConfig, getNetworkConfig, getSeedPhrase, writeDeployConfig } from "../config";

export class Wallet {
    public static async create(target: Target, env: Environment):Promise<Wallet> {

        const networkConfig = await getNetworkConfig(env === "testnet" ? `${target}_testnet`: `${target}_local`);

        const {addr_prefix, rpc_url, gas_price, denom} = networkConfig;


        const signer = await DirectSecp256k1HdWallet.fromMnemonic(
            getSeedPhrase(), 
            { 
                prefix: addr_prefix,
            }
        );


        const accounts = await signer.getAccounts()
        const address = accounts[0].address

        const client = await SigningCosmWasmClient.connectWithSigner(
            rpc_url,
            signer,
            { 
                gasPrice: GasPrice.fromString(`${gas_price}${denom}`), 
            }
        );

        const account = await client.getAccount(address);
        if(!account) {
            console.warn(`Account ${address} needs funds for executions`);
        }

        return new Wallet(env, signer, client, address, networkConfig);
    }

    public async balance():Promise<number> {
        const coin = await this.client.getBalance(this.address, this.networkConfig.denom);
        return Number(coin.amount)
    }


    public async instantiateContract(name: ContractName, instantiate_msg: any) {
        const deployConfig = await getDeployConfig(name, this.env);

        if(!deployConfig.codeId) {
            throw new Error("Contract needs to be uploaded before it can be instantiated!");
        }

        const result = await this.client.instantiate(
            this.address,
            deployConfig.codeId,
            instantiate_msg,
            name,
            "auto",
            {
                admin: this.address
            }
        )

        const { contractAddress } = result;
        if(!contractAddress || contractAddress === "") {
            throw new Error("Failed to instantiate contract");
        }

        console.log("instantiated", name, "at", contractAddress);

        deployConfig.address = contractAddress;
        await writeDeployConfig(name, this.env, deployConfig);
    }

    public async migrateContract(name: ContractName, codeId: number, hashHex: string, migrate_msg: any) {
        const {address} = await getDeployConfig(name, this.env);


        if(!address) {
            throw new Error("Contract needs to have a pre-existing address before it can be migrated!");
        }

        await this.client.migrate(
            this.address,
            address,
            codeId,
            migrate_msg,
            "auto",
        )

        const deployConfig = await getDeployConfig(name, this.env);
        deployConfig.hash = hashHex;
        deployConfig.codeId = codeId;

        await writeDeployConfig(name, this.env, deployConfig); 

        console.log("migrated", name, "at", deployConfig.address);
    }


//migrate(senderAddress: string, contractAddress: string, codeId: number, migrateMsg: JsonObject, fee: StdFee | "auto" | number, memo?: string): Promise<MigrateResult>;

    public async setIbcPort(name: ContractName) {
        const deployConfig = await getDeployConfig(name, this.env);

        if(!deployConfig.address) {
            throw new Error("Contract needs to be instantiated before it get the ibc port!");
        }
        const contractInfo = await this.client.getContract(deployConfig.address);

        if(!contractInfo.ibcPortId) {
            throw new Error("Contract does not have an ibc port");
        }

        console.log("Setting ibc port for", name, "to", contractInfo.ibcPortId);

        deployConfig.ibcPort = contractInfo.ibcPortId;



        await writeDeployConfig(name, this.env, deployConfig);
    }

    // returns true if it uploaded a new code id, otherwise false
    public async uploadContract(name: ContractName, kind: "migrate" | "deploy"):Promise<{codeId: number, isNew: boolean, hashHex: string}> {
        const data = await fs.readFile(getContractPath(name));
        const hashBuffer = await crypto.subtle.digest("SHA-256", data)
        const hashArray = Array.from(new Uint8Array(hashBuffer)); // convert buffer to byte array
        const hashHex = hashArray
            .map((b) => b.toString(16).padStart(2, "0"))
            .join(""); // convert bytes to hex string

        const deployConfig = await getDeployConfig(name, this.env);
        if(kind === "migrate") {
            console.log(`Contract ${name} needs a migration, uploading...`);
        } else if(deployConfig.hash && deployConfig.codeId && deployConfig.hash === hashHex) {
            try {
                const contractDetails = await this.client.getCodeDetails(deployConfig.codeId);
                if(contractDetails.id === deployConfig.codeId) {
                    console.log(`Contract ${name} already uploaded, code id is ${deployConfig.codeId}`);
                } else {
                    throw new Error("Code ID does not match");
                }
                return {codeId: deployConfig.codeId, isNew: false, hashHex};
            } catch(e) {
                console.log(`Contract ${name} codeId is nonexistant or changed, uploading...`);
            }
        } else {
            console.log(`Contract ${name} has changed, uploading...`);
        }


        const uploadReceipt = await this.client.upload(this.address, data, "auto");
        const {codeId} = uploadReceipt;

        if(Number.isNaN(codeId)) {
            throw new Error("Failed to upload contract");
        }

        console.log(`Contract uploaded with code ID ${codeId}`);

        if(kind === "deploy") {
            deployConfig.hash = hashHex;
            deployConfig.codeId = codeId;

            await writeDeployConfig(name, this.env, deployConfig); 
        }

        return {codeId, isNew: true, hashHex};
    }


    public async queryContract<T>(contractAddress, msg):Promise<T> {
        return await this.client.queryContractSmart(contractAddress, msg);
    }

    public async execContract(contractAddress, msg, fee: StdFee | "auto" | number = "auto", memo?: string, funds?: readonly Coin[]):Promise<ExecuteResult> {
        return await this.client.execute(this.address, contractAddress, msg, fee, memo, funds);
    }

    public async execContracts(instructions: ExecuteInstruction[], fee: StdFee | "auto" | number = "auto", memo?: string):Promise<ExecuteResult> {
        return await this.client.executeMultiple(this.address, instructions, fee, memo);
    }

    private constructor(public readonly env: Environment, public readonly signer: DirectSecp256k1HdWallet, public readonly client: SigningCosmWasmClient, public readonly address: string, public readonly networkConfig: NetworkConfig) {
    }
}