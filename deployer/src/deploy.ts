import { ContractName, Environment, ONLY_IF_NEW } from "./config";
import { getEnvironment } from "./utils/args";
import { Wallet } from "./utils/wallet";

export async function deploy() {
    const env = getEnvironment();

    const neutronWallet = await Wallet.create("neutron", env);
    console.log(`Neutron wallet address: ${neutronWallet.address}, balance: ${await neutronWallet.balance()}`);

    const kujiraWallet = await Wallet.create("kujira", env);
    console.log(`Kujira wallet address: ${kujiraWallet.address}, balance: ${await kujiraWallet.balance()}`);

    const stargazeWallet = await Wallet.create("stargaze", env);
    console.log(`Stargaze wallet address: ${stargazeWallet.address}, balance: ${await stargazeWallet.balance()}`);

    await deployContract(neutronWallet, "warehouse");
    await deployContract(kujiraWallet, "payment");
    await deployContract(stargazeWallet, "nft");
}

async function deployContract(wallet: Wallet, name: ContractName) {
    console.log(``);
    const {isNew} = await wallet.uploadContract(name, "deploy-always" /*"deploy-if-new"*/);
    if(isNew || !ONLY_IF_NEW) {
        await wallet.instantiateContract(name, {
        });

        await wallet.setIbcPort(name);
    } else {
        console.log(`${name} contract already uploaded, not deploying again`)
    }
}