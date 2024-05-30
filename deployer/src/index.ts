import { deploy } from "./deploy";

(async () => {
    await deploy();
})().catch(console.error);