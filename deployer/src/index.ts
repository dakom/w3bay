import { deploy } from "./deploy";
import { migrate } from "./migrate";
import { getAction } from "./utils/args";

(async () => {
    const action = getAction();
    switch(action) {
        case "deploy":
            await deploy();
            break;
        case "migrate":
            await migrate();
            break;
        // exhaustive check
        default:
            return action satisfies never;
    }
})().catch(console.error);