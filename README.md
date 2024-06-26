# Web3Bay (Hackathon Submission by dakom)

Ever bought an orange? How about a thousand oranges?

Prices go down with wholesale, but most people buy retail... what if you could collaboratively shop, and guarantee all the dynamics with refunds, bad actors, etc.?

Think Ebay, Aliexpress, etc. but using Web3 tech to make smart contract driven guarantees with multiple shoppers.

Overall, consumers get better prices, and merchants benefit with consistent margins.

[LIVE FRONTEND](https://dakom.github.io/w3bay/) (requires a running relayer!)

# Developers

* [Auto-generated Rustdoc](https://dakom.github.io/w3bay/doc/shared/) for all shared types, contract messages, including IBC and events (from the `shared` package)
* Fullstack Rust, types are shared between contracts and frontend (even getting strongly typed events with From/Into impls!)
* Taskfile with simple commands to make development a breeze
* Shared config, one file to configure the network, one auto-generated file to maintain contract addresses, ibc ports, etc.

## Getting Started

### Prerequisites

* [Rust](https://www.rust-lang.org/)
* [Go](https://go.dev/)
* [npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) (nodejs package manager)
* [Taskfile](https://taskfile.dev) (overall task runner) 
* [jq](https://jqlang.github.io/jq/download/) (parse json on commandline)
* [Trunk](https://trunkrs.dev/) (for frontend dev/deploy)
* [http-server-rs](https://github.com/http-server-rs/http-server) (for frontend local media serving)
* anything else the commandline tells you to install :)

### Setup

#### Local chains

1. Create docker images for chains:
   - Neutron: 
      - clone [https://github.com/neutron-org/neutron](https://github.com/neutron-org/neutron)
      - `docker buildx build --load --build-context app=. -t local-neutron --build-arg BINARY=neutrond .`
   - Kujira
      - clone [https://github.com/Team-Kujira/pond-images](https://github.com/Team-Kujira/pond-images)
      - in `kujira/` subdirectory: `docker build --build-arg go_version=1.21.3 --build-arg kujira_version=03985a2 -t local-kujira .`
      - the exact go and kujira versions are taken from https://github.com/Team-Kujira/pond-images/blob/main/versions.yml
   - Stargaze
      - clone [https://github.com/public-awesome/stargaze](https://github.com/public-awesome/stargaze)
      - `docker build -t local-stargaze .`
2. Create containers for chains: `task create-chains`
3. Start chains: `task start-chains`

Then you can stop and start the local chains as-needed with `task stop-chains` / `task start-chains`

Debugging individual chains by shell can be done via `task sh-chain-[neutron|kujira|stargaze]`.


#### High-Level / easy setup 

If targeting locally, make sure you've started the local chains (see `Local Chains` above)

If you already have all the depenencies, wallet setup, etc., then it's as easy as:

1. `task contracts-deploy-[local|testnet]`
2. `task relayer-setup-[local|testnet]`

That's all for initial setup, then, to get a live working environment

1. `task relayer-start-[local|testnet]` (in its own terminal) 
2. `task frontend-dev-[local|testnet]` (in its own terminal) 

As contracts change and you want to migrate, just `task contracts-migrate-[local|testnet]` and hard reload the frontend

If you need a full redeployment (new addresses, wipe the state, etc.):

1. shut down the "live working environment" (kill the terminals) 
2. `task contracts-deploy-[local|testnet]`
3. `task relayer-create-channels-[local|testnet]`
4. Restart the "live working environment" as above

#### Low-level / granular setup setup 

Step-by-step instructions to allow for debugging individual commands etc.

If targeting locally, make sure you've started the local chains (see `Local Chains` above)

1. (one-time) make sure you have all the testnets installed available in Keplr
   - Neutron: https://neutron.celat.one/pion-1 and hit "connect wallet"
   - Kujira: https://github.com/SynergyNodes/Add-Kujira-Testnet-to-Keplr (maybe use Polkachu RPC nodes instead, as in the network.json file here)
   - Stargaze: https://violetboralee.medium.com/stargaze-network-how-to-add-stargaze-testnet-to-keplr-cosmostation-leap-and-get-test-stars-5a6ae2ca494f
   - you may then need to go to keplr settings and "adjust chain visibility" to see balance / check address / etc.
2. (as-needed) get some testnet tokens
   - Neutron: https://docs.neutron.org/neutron/faq/#where-is-the-testnet-faucet
   - Kujira: via the #public-testnet-faucet channel on Discord
   - Stargaze: https://violetboralee.medium.com/stargaze-network-how-to-add-stargaze-testnet-to-keplr-cosmostation-leap-and-get-test-stars-5a6ae2ca494f
      - May need to manually add the #faucet channel
3. (one-time) edit .env.example to add your seed phrases (needed for deploying contracts, local chains, running the relayer, not as a regular user), and rename to .env
4. (one-time) setup the relayer
   - Install the Go relayer: https://github.com/cosmos/relayer
   - Initialize the Go relayer: `rly config init`
   - Configure the chains: `task relayer-add-chains-[local|testnet]`
   - Configure the wallet: `task relayer-add-wallet-[local|testnet]`
   - Make sure the wallet has funds: `task relayer-check-wallet-[local|testnet]`
      - each chain should have non-zero balance
   - Create paths: `task relayer-create-paths-[local|testnet]`
   - Create clients: `task relayer-create-clients-[local|testnet]`
   - Create connections: `task relayer-create-connections-[local|testnet]`
   - Create channels: gotcha! don't do that yet :) it will read the deploy file to get the ibc ports, so we do that after deploying contracts
5. (one-time) Install npm dependencies
   - in `deployer`, run `npm install`
6. (one-time and hacking) build and deploy contracts
   - `task contracts-deploy-[local|testnet]` (this will use `native` build mode... to avoid this, see subcommands in Taskfile)
   - `task relayer-create-channels-[local|testnet]`
7. (one-time and hacking) run the frontend locally
   - `task frontend-dev-[local|testnet]` (in its own terminal)
8. (one-time and hacking) start relaying
   - `task relayer-start-[local|testnet]` (in its own terminal)

The order in all the above is somewhat important, but once you're off to the races, different parts can be iterated (e.g. redeploying contracts and recreating ibc channels)

## High-level IBC Flow

Kujira <-> Neutron <-> Stargaze

* Pay on Kujira, relays message to Neutron to purchase an item
* Neutron reacts to Kujira, and sends message to Stargaze for mintng NFTs
* On shipping, cost-saving refunds are calculated on Neutron and sent to the user on Kujira
* Removing a purchase goes through burning the NFT on Stargaze, which sends the IBC message to Neutron, which then sends another IBC message to Kujira for the final refund

The Neutron Warehouse contract accomplishes this via multiple channels to both sides.

## Deploy flow

A deploy.json is written to locally, keeping addresses, ports, and hash digests in sync.

This allows simply deploying from commandline and then importing the same config file in the frontend.

If contracts don't change, it checks the hash to avoid rebuilding

# Configuration

The root-level `network.json` file is used to configure both the frontend *and* deploy scripts
Similarly, the root-level `deploy.json` is written to from the deploy scripts (do not edit by hand!) and imported by frontend
For hacking around, frontend has its own config.rs for things like jumping straight into a page to debug, etc.

## Frontend

* `task frontend-dev` to start a local server and hack away
* Deployment is done to github pages via CI. Can be run manually too of course (see CI commands)

Strings are handled via Fluent project, different languages can be added (currently English and Hebrew-ish)
(this was only partially done due to time, but it's all setup)

Rust bindings to wallets are done via global-level UMD imports

Those UMD scripts are from:

* CosmWasmJS: https://github.com/dakom/CosmWasmJS
   - pretty much just re-exports the native cosmjs modules since the official project is no longer maintained

## Contracts

* Contracts are built via `task contracts-build`. For the sake of faster build times, `task contracts-build-native` can be run to avoid docker, but it requires all the tools be available (e.g. binaryen / wasm-opt)

* Contracts are deployed via `task contracts-only-deploy`. However, the typical way is `task contracts-deploy-[local|testnet]` will build _and_ deploy in one step, using native build mode (not docker). Same thing goes for migrations (just substitute "migrate" for "deploy").


There is no automated testing setup at the moment, but it would be trivial to add. Testing is currently done gamedev style, by playtesting ;)
