import * as path from "path";
import * as fs from "fs-extra"; 
import * as dotenv from "dotenv";
import { getArg } from "./utils/args";
import { GasPrice } from "@cosmjs/stargate";

dotenv.config({ path: path.resolve('../.env') });

export const NETWORK_PATH = "../network.json";
export const DEPLOY_PATH = "../deploy.json";
export const WASM_ARTIFACTS_PATH = "../wasm/artifacts";

export type Target = "neutron" | "stargaze" | "kujira";
export type ContractName = "warehouse" | "payment" | "nft"

export interface NetworkConfig {
    target: Target;
    rpc_url: string;
    rest_url: string;
    gas_price;
    full_denom: string;
    denom: string;
    chain_id: string;
    addr_prefix: string;
}

export function getSeedPhrase() {
    return process.env.COSMOS_SEED_PHRASE as string;
}
export async function getNetworkConfig(target: Target):Promise<NetworkConfig> {
    console.log(path.resolve(NETWORK_PATH));
    const ALL_CONFIG = JSON.parse(await fs.readFile(path.resolve(NETWORK_PATH), "utf8"));
    const config = ALL_CONFIG[target];
    return { 
        target,  
        ...config
    };
}

export function getContractPath(contractName: ContractName):string {
    return path.resolve(WASM_ARTIFACTS_PATH, `${contractName}.wasm`);
}

export interface DeployConfig {
    name: ContractName;
    hash?: string
    codeId?: number
    address?: string
    ibcPort?: string
}
export async function getDeployConfig(contractName: ContractName):Promise<DeployConfig> {
    const ALL_CONFIG = JSON.parse(await fs.readFile(path.resolve(DEPLOY_PATH), "utf8"));
    const config = ALL_CONFIG[contractName];
    return {
        name: contractName,
        ...config
    }
}

export async function writeDeployConfig(contractName: ContractName, config: DeployConfig) {
    const ALL_CONFIG = JSON.parse(await fs.readFile(path.resolve(DEPLOY_PATH), "utf8"));
    ALL_CONFIG[contractName] = config;
    await fs.writeFile(path.resolve(DEPLOY_PATH), JSON.stringify(ALL_CONFIG, null, 2));
}

export function getTarget():Target {
    const target = getArg("target");

    if (target === "neutron" || target === "stargaze" || target === "kujira") {
        return target;
    }

    throw new Error("Please specify a target with --target=[neutron|stargaze|kujira]");

}