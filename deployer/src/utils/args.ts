import { Environment } from "src/config";

export function getArg(key:string) {
    for (const arg of process.argv) {
        if (arg.startsWith(`--${key}=`)) {
            const value = arg.substring(key.length + 3);
            return (!value || value === "") ? null : value;
        }
    }

    return null; 
}

export function getEnvironment():Environment {
    const env = getArg("CHAINENV");

    if (env === "testnet" || env === "local") {
        return env;
    }

    throw new Error("Please specify an environment with --CHAINENV=[testnet|local]");

}