export function getArg(key:string) {
    for (const arg of process.argv) {
        if (arg.startsWith(`--${key}=`)) {
            const value = arg.substring(key.length + 3);
            return (!value || value === "") ? null : value;
        }
    }

    return null; 
}