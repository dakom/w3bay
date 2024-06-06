import { ContractName, Environment, ONLY_IF_NEW } from "./config";
import { getEnvironment } from "./utils/args";
import { Wallet } from "./utils/wallet";

export async function migrate() {
    const env = getEnvironment();

    const neutronWallet = await Wallet.create("neutron", env);
    console.log(`Neutron wallet address: ${neutronWallet.address}, balance: ${await neutronWallet.balance()}`);

    const kujiraWallet = await Wallet.create("kujira", env);
    console.log(`Kujira wallet address: ${kujiraWallet.address}, balance: ${await kujiraWallet.balance()}`);

    const stargazeWallet = await Wallet.create("stargaze", env);
    console.log(`Stargaze wallet address: ${stargazeWallet.address}, balance: ${await stargazeWallet.balance()}`);

    await migrateContract(neutronWallet, "warehouse");
    await migrateContract(kujiraWallet, "payment");
    await migrateContract(stargazeWallet, "nft");
}

async function migrateContract(wallet: Wallet, name: ContractName) {
    console.log(``);
    const {hashHex, codeId} = await wallet.uploadContract(name, "migrate");
    await wallet.migrateContract(name, codeId, hashHex, { });
}
