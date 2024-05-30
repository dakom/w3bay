import { ContractName } from "./config";
import { Wallet } from "./utils/wallet";

export async function deploy() {
    const neutronWallet = await Wallet.create("neutron");
    console.log(`Neutron wallet address: ${neutronWallet.address}, balance: ${await neutronWallet.balance()}`);

    const kujiraWallet = await Wallet.create("kujira");
    console.log(`Kujira wallet address: ${kujiraWallet.address}, balance: ${await kujiraWallet.balance()}`);

    const stargazeWallet = await Wallet.create("stargaze");
    console.log(`Stargaze wallet address: ${stargazeWallet.address}, balance: ${await stargazeWallet.balance()}`);

    await deployContract(neutronWallet, "warehouse");
    await deployContract(kujiraWallet, "payment");
    await deployContract(stargazeWallet, "nft");
}

async function deployContract(wallet: Wallet, name: ContractName, onlyIfNew: boolean = false) {
    const isNew = await wallet.uploadContract(name);
    if(isNew || !onlyIfNew) {
        await wallet.instantiateContract(name, {
        });

        await wallet.setIbcPort(name);
    } else {
        console.log(`${name} contract already uploaded, not deploying again`)
    }
}