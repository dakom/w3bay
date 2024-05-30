use cosmwasm_std::{to_json_binary, Addr, Binary, BlockInfo, IbcMsg, IbcTimeout, Order, QueryResponse, Storage};
use cw_storage_plus::{Bound, Item, Map};
use cw_utils::Expiration;
use shared::{
    ibc::TIMEOUT_SECONDS, msg::{contract::{
        nft::{
            events::{
                ApprovalEvent, ApproveAllEvent, BurnEvent, MintEvent, RevokeAllEvent, RevokeEvent,
                TransferEvent,
            }, AllNftInfoResponse, Approval, ApprovalResponse, ApprovalsResponse, Cw721ReceiveMsg, ExecuteMsg, FullTokenInfo, Metadata, NftContractInfo, NftInfoAndIdResponse, NftInfoResponse, NumTokensResponse, OperatorsResponse, OwnerOfResponse, QueryMsg, TokensResponse
        }, 
        warehouse::IbcExecuteMsg as WarehouseIbcExecuteMsg
    }, purchase::PurchaseId}, response::QueryResponseExt};
use anyhow::{bail, Context, Result};

use super::{State, StateContext};

// this is unbounded, but that's how it is in the cw721 reference too
// https://github.com/CosmWasm/cw-nfts/blob/bf70cfb516b39a49db423a4b353c2bb8518c2b51/contracts/cw721-base/src/state.rs#L108
const APPROVALS: Map<&str, Vec<Approval>> = Map::new("nft-approvals");
const OPERATORS: Map<(Addr, Addr), Expiration> = Map::new("nft-operators");
const TOKEN_COUNT: Item<u64> = Item::new("nft-count");
const TOKEN_ID_CURSOR: Item<u64> = Item::new("nft-token-cursor");
const OWNER_TO_TOKEN_IDS: Map<(Addr, String), u8> = Map::new("nft-owners");
const TOKEN_IDS: Map<&str, u8> = Map::new("nft-ids");
const TOKEN_META: Map<&str, Metadata> = Map::new("nft-metadata");
const TOKEN_OWNER: Map<&str, Addr> = Map::new("nft-owner");
const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 100;

impl State<'_> {
    pub(crate) fn nft_mint(
        &self,
        ctx: &mut StateContext,
        owner: Addr,
        meta: Metadata,
    ) -> Result<()> {
        let token_id = TOKEN_ID_CURSOR.may_load(ctx.store)?.unwrap_or_default();

        self.nft_mint_inner(ctx, owner, token_id.to_string(), meta)?;

        TOKEN_ID_CURSOR.save(ctx.store, &(token_id + 1))?;

        Ok(())
    }
    pub(crate) fn nft_handle_query(
        &self,
        store: &dyn Storage,
        msg: QueryMsg,
    ) -> Result<QueryResponse> {
        match msg {
            QueryMsg::Info {  } => {
                unreachable!("already handled")
            }
            QueryMsg::IbcChannel {} => {
                unreachable!("already handled")
            }
            QueryMsg::ContractInfo {} => self.nft_contract_info(store)?.query_result(),

            QueryMsg::NftInfo { token_id } => self.nft_info(store, &token_id)?.query_result(),
            QueryMsg::NftInfos { token_ids } => {
                let mut nfts = vec![];
                for token_id in token_ids {
                    nfts.push(NftInfoAndIdResponse {
                        extension: self.nft_info(store, &token_id)?.extension,
                        token_id,
                    });
                }
                nfts.query_result()
            }

            QueryMsg::OwnerOf {
                token_id,
                include_expired,
            } => self
                .nft_owner_of(store, &token_id, include_expired.unwrap_or(false))?
                .query_result(),

            QueryMsg::AllNftInfo {
                token_id,
                include_expired,
            } => self
                .nft_all_info(store, &token_id, include_expired.unwrap_or(false))?
                .query_result(),

            QueryMsg::AllOperators {
                owner,
                include_expired,
                start_after,
                limit,
            } => self
                .nft_operators(
                    store,
                    self.api.addr_validate(&owner)?,
                    include_expired.unwrap_or(false),
                    cw_utils::maybe_addr(self.api, start_after)?,
                    limit,
                )?
                .query_result(),

            QueryMsg::NumTokens {} => self.nft_num_tokens(store)?.query_result(),

            QueryMsg::Tokens {
                owner,
                start_after,
                limit,
            } => {
                let owner = self.api.addr_validate(&owner)?;

                TokensResponse {
                    tokens: self.nft_map_token_ids(
                        store,
                        Some(owner),
                        start_after,
                        limit,
                        |id| id,
                    )?,
                }
                .query_result()
            }

            QueryMsg::AllTokens { start_after, limit } => TokensResponse {
                tokens: self.nft_map_token_ids(store, None, start_after, limit, |id| id)?,
            }
            .query_result(),

            QueryMsg::Approval {
                token_id,
                spender,
                include_expired,
            } => self
                .nft_approval(
                    store,
                    &token_id,
                    self.api.addr_validate(&spender)?,
                    include_expired.unwrap_or(false),
                )?
                .query_result(),

            QueryMsg::Approvals {
                token_id,
                include_expired,
            } => self
                .nft_approvals(store, &token_id, include_expired.unwrap_or(false))?
                .query_result(),
        }
    }

    fn nft_token_full(&self, store: &dyn Storage, token_id: &str) -> Result<FullTokenInfo> {
        let owner = TOKEN_OWNER.load(store, token_id)?;
        let approvals = self.nft_token_approvals(store, token_id)?;
        let meta = TOKEN_META.load(store, token_id)?;

        Ok(FullTokenInfo {
            owner,
            approvals,
            extension: meta,
        })
    }

    fn nft_map_token_ids<A, F>(
        &self,
        store: &dyn Storage,
        owner: Option<Addr>,
        start_after: Option<String>,
        limit: Option<u32>,
        f: F,
    ) -> Result<Vec<A>>
    where
        F: Fn(String) -> A + Clone,
    {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

        match owner {
            Some(owner) => {
                let min: Option<Bound<String>> = start_after.map(Bound::exclusive);
                let vec = OWNER_TO_TOKEN_IDS
                    .prefix(owner)
                    .keys(store, min, None, cosmwasm_std::Order::Ascending)
                    .take(limit)
                    .map(|res| res.map(f.clone()).map_err(|err| err.into()))
                    .collect::<Result<Vec<A>>>()?;

                Ok(vec)
            }
            None => {
                let min: Option<Bound<&str>> =
                    start_after.as_ref().map(|s| Bound::exclusive(s.as_str()));
                let vec = TOKEN_IDS
                    .keys(store, min, None, cosmwasm_std::Order::Ascending)
                    .take(limit)
                    .map(|res| res.map(f.clone()).map_err(|err| err.into()))
                    .collect::<Result<Vec<A>>>()?;

                Ok(vec)
            }
        }
    }

    fn nft_token_approvals(&self, store: &dyn Storage, token_id: &str) -> Result<Vec<Approval>> {
        APPROVALS.load(store, token_id).map_err(|err| err.into())
    }

    fn nft_contract_info(&self, _store: &dyn Storage) -> Result<NftContractInfo> {
        Ok(NftContractInfo {
            name: "demo".to_string(),
            symbol: "demo".to_string(),
        })
    }

    fn nft_info(&self, store: &dyn Storage, token_id: &str) -> Result<NftInfoResponse> {
        let token = self.nft_token_full(store, token_id)?;

        Ok(NftInfoResponse {
            extension: token.extension,
        })
    }

    fn nft_all_info(
        &self,
        store: &dyn Storage,
        token_id: &str,
        include_expired: bool,
    ) -> Result<AllNftInfoResponse> {
        let token = self.nft_token_full(store, token_id)?;

        Ok(AllNftInfoResponse {
            access: OwnerOfResponse {
                owner: token.owner,
                approvals: filter_approvals(&self.env.block, &token.approvals, include_expired),
            },
            info: NftInfoResponse {
                extension: token.extension,
            },
        })
    }

    fn nft_owner_of(
        &self,
        store: &dyn Storage,
        token_id: &str,
        include_expired: bool,
    ) -> Result<OwnerOfResponse> {
        let approvals = self.nft_token_approvals(store, token_id)?;

        Ok(OwnerOfResponse {
            owner: TOKEN_OWNER.load(store, token_id)?,
            approvals: filter_approvals(&self.env.block, &approvals, include_expired),
        })
    }

    fn nft_operators(
        &self,
        store: &dyn Storage,
        owner: Addr,
        include_expired: bool,
        start_addr: Option<Addr>,
        limit: Option<u32>,
    ) -> Result<OperatorsResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_addr.map(Bound::exclusive);

        let res: Result<Vec<_>> = OPERATORS
            .prefix(owner)
            .range(store, start, None, Order::Ascending)
            .filter(|r| {
                // same unwrap in reference contract: https://github.com/CosmWasm/cw-nfts/blob/bf70cfb516b39a49db423a4b353c2bb8518c2b51/contracts/cw721-base/src/query.rs#L80
                include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&self.env.block)
            })
            .take(limit)
            .map(|res| {
                let (spender, expires) = res?;
                Ok(Approval { spender, expires })
            })
            .collect();
        Ok(OperatorsResponse { operators: res? })
    }

    fn nft_num_tokens(&self, store: &dyn Storage) -> Result<NumTokensResponse> {
        let count = TOKEN_COUNT.may_load(store)?.unwrap_or_default();

        Ok(NumTokensResponse { count })
    }

    fn nft_approval(
        &self,
        store: &dyn Storage,
        token_id: &str,
        spender: Addr,
        include_expired: bool,
    ) -> Result<ApprovalResponse> {
        let owner = TOKEN_OWNER.load(store, token_id)?;
        let approvals = self.nft_token_approvals(store, token_id)?;

        // token owner has absolute approval

        if owner == spender {
            let approval = Approval {
                spender: owner,
                expires: Expiration::Never {},
            };
            return Ok(ApprovalResponse { approval });
        }

        let filtered: Vec<_> = approvals
            .into_iter()
            .filter(|t| t.spender == spender)
            .filter(|t| include_expired || !t.is_expired(&self.env.block))
            .collect();

        if filtered.is_empty() {
            bail!("approval not found");
        }
        // we expect only one item
        let approval = filtered[0].clone();

        Ok(ApprovalResponse { approval })
    }

    /// approvals returns all approvals owner given access to
    fn nft_approvals(
        &self,
        store: &dyn Storage,
        token_id: &str,
        include_expired: bool,
    ) -> Result<ApprovalsResponse> {
        let approvals = self.nft_token_approvals(store, token_id)?;

        let approvals: Vec<_> = approvals
            .into_iter()
            .filter(|t| include_expired || !t.is_expired(&self.env.block))
            .collect();

        Ok(ApprovalsResponse { approvals })
    }

    pub(crate) fn nft_handle_exec(
        &self,
        ctx: &mut StateContext,
        msg_sender: Addr,
        msg: ExecuteMsg,
    ) -> Result<()> {
        match msg {
            ExecuteMsg::Burn { token_id } => {
                self.nft_burn(ctx, msg_sender, token_id)
            }, 
            ExecuteMsg::Approve {
                spender,
                token_id,
                expires,
            } => self.nft_approve(
                ctx,
                &msg_sender,
                &self.api.addr_validate(&spender)?,
                &token_id,
                expires,
            ),
            ExecuteMsg::Revoke { spender, token_id } => {
                self.nft_revoke(ctx, &msg_sender, &self.api.addr_validate(&spender)?, &token_id)
            }

            ExecuteMsg::ApproveAll { operator, expires } => {
                self.nft_approve_all(ctx, &msg_sender, &self.api.addr_validate(&operator)?, expires)
            }

            ExecuteMsg::RevokeAll { operator } => {
                self.nft_revoke_all(ctx, &msg_sender, &self.api.addr_validate(&operator)?)
            }

            ExecuteMsg::TransferNft {
                recipient,
                token_id,
            } => self.nft_transfer(ctx, &msg_sender, &self.api.addr_validate(&recipient)?, &token_id),

            ExecuteMsg::SendNft {
                contract,
                token_id,
                msg,
            } => self.nft_send(
                ctx,
                &msg_sender,
                &self.api.addr_validate(&contract)?,
                &token_id,
                msg,
            ),
        }
    }

    fn nft_mint_inner(
        &self,
        ctx: &mut StateContext,
        owner: Addr,
        token_id: String,
        meta: Metadata,
    ) -> Result<()> {
        TOKEN_IDS.save(ctx.store, &token_id, &1)?;
        OWNER_TO_TOKEN_IDS.save(ctx.store, (owner.clone(), token_id.clone()), &1)?;
        APPROVALS.save(ctx.store, &token_id, &Vec::new())?;
        TOKEN_OWNER.save(ctx.store, &token_id, &owner)?;
        TOKEN_META.save(ctx.store, &token_id, &meta)?;

        self.nft_increment_tokens(ctx)?;

        ctx.response_mut().add_event(MintEvent { owner, token_id });

        Ok(())
    }

    pub(crate) fn nft_burn(
        &self,
        ctx: &mut StateContext,
        msg_sender: Addr,
        token_id: String,
    ) -> Result<()> {
        let approvals = self.nft_token_approvals(ctx.store, &token_id)?;

        // ensure we have permissions
        self.nft_check_can_send(ctx.store, &msg_sender, &token_id, &approvals)?;

        let owner = TOKEN_OWNER.load(ctx.store, &token_id)?;
        let meta = TOKEN_META.load(ctx.store, &token_id)?;

        TOKEN_IDS.remove(ctx.store, &token_id);
        OWNER_TO_TOKEN_IDS.remove(ctx.store, (owner, token_id.clone()));
        APPROVALS.remove(ctx.store, &token_id);
        TOKEN_OWNER.remove(ctx.store, &token_id);
        TOKEN_META.remove(ctx.store, &token_id);

        self._nft_decrement_tokens(ctx)?;

        ctx.response_mut().add_event(BurnEvent { token_id });

        let purchase_id = meta
            .attributes
            .and_then(|attributes| attributes
                .iter()
                .find_map(|t| {
                    if t.trait_type == "purchase-id" {
                        t.value.parse::<PurchaseId>().ok()
                    } else {
                        None
                    }
                })
            )
            .context("missing purchase_id")?;

        let msg = WarehouseIbcExecuteMsg::RemovePurchase { id: purchase_id };

        // outbound IBC message, where packet is then received on other chain
        let channel_id = self
            .get_ibc_channel(ctx.store)?
            .endpoint
            .channel_id;

        ctx.response_mut().add_message(IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&msg)?,
            timeout: IbcTimeout::with_timestamp(self.env.block.time.plus_seconds(TIMEOUT_SECONDS)),
        });


        Ok(())
    }

    pub(crate) fn nft_transfer(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        recipient: &Addr,
        token_id: &str,
    ) -> Result<()> {
        self.nft_transfer_inner(ctx, msg_sender, recipient, token_id)?;

        Ok(())
    }

    pub(crate) fn nft_approve(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        spender: &Addr,
        token_id: &str,
        expires: Option<Expiration>,
    ) -> Result<()> {
        self.nft_update_approvals(ctx, msg_sender, spender, token_id, true, expires)?;
        Ok(())
    }

    pub(crate) fn nft_revoke(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        spender: &Addr,
        token_id: &str,
    ) -> Result<()> {
        self.nft_update_approvals(ctx, msg_sender, spender, token_id, false, None)?;
        Ok(())
    }

    pub(crate) fn nft_approve_all(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        operator: &Addr,
        expires: Option<Expiration>,
    ) -> Result<()> {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&self.env.block) {
            bail!("expired");
        }

        // set the operator for us
        OPERATORS.save(
            ctx.store,
            (msg_sender.clone(), operator.clone()),
            &expires,
        )?;

        ctx.response_mut().add_event(ApproveAllEvent {
            operator: operator.clone(),
            expires,
        });

        Ok(())
    }

    fn nft_revoke_all(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        operator: &Addr,
    ) -> Result<()> {
        OPERATORS.remove(ctx.store, (msg_sender.clone(), operator.clone()));

        ctx.response_mut().add_event(RevokeAllEvent {
            operator: operator.clone(),
        });

        Ok(())
    }

    fn nft_update_approvals(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        spender: &Addr,
        token_id: &str,
        // if add == false, remove. if add == true, remove then set with this expiration
        add: bool,
        expires: Option<Expiration>,
    ) -> Result<Vec<Approval>> {
        let mut approvals = self.nft_token_approvals(ctx.store, token_id)?;
        // ensure we have permissions
        self.nft_check_can_approve(ctx.store, msg_sender, token_id)?;

        // update the approval list (remove any for the same spender before adding)
        approvals.retain(|apr| apr.spender != *spender);

        // only difference between approve and revoke
        if add {
            // reject expired data as invalid
            let expires = expires.unwrap_or_default();
            if expires.is_expired(&self.env.block) {
                bail!("expired");
            }
            let approval = Approval {
                spender: spender.clone(),
                expires,
            };
            approvals.push(approval);

            ctx.response_mut().add_event(ApprovalEvent {
                spender: spender.clone(),
                token_id: token_id.to_string(),
                expires,
            });
        } else {
            ctx.response_mut().add_event(RevokeEvent {
                spender: spender.clone(),
                token_id: token_id.to_string(),
            });
        }

        APPROVALS.save(ctx.store, token_id, &approvals)?;

        Ok(approvals)
    }

    fn nft_check_can_approve(
        &self,
        store: &dyn Storage,
        msg_sender: &Addr,
        token_id: &str,
    ) -> Result<()> {
        let owner = TOKEN_OWNER.load(store, token_id)?;
        // owner can approve
        if owner == *msg_sender {
            return Ok(());
        }
        // operator can approve
        let op = OPERATORS.may_load(store, (owner, msg_sender.clone()))?;
        match op {
            Some(ex) => {
                if ex.is_expired(&self.env.block) {
                    bail!("expired");
                } else {
                    Ok(())
                }
            }
            None => {
                bail!("auth");
            }
        }
    }

    fn nft_send(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        contract: &Addr,
        token_id: &str,
        msg: Binary,
    ) -> Result<()> {
        // Transfer token
        self.nft_transfer_inner(ctx, msg_sender, contract, token_id)?;

        let msg = Cw721ReceiveMsg {
            sender: msg_sender.to_string(),
            token_id: token_id.to_string(),
            msg,
        };

        ctx.response_mut()
            .add_execute_submessage_oneshot(contract, &msg)?;

        Ok(())
    }

    fn nft_transfer_inner(
        &self,
        ctx: &mut StateContext,
        msg_sender: &Addr,
        recipient: &Addr,
        token_id: &str,
    ) -> Result<()> {
        let approvals = self.nft_token_approvals(ctx.store, token_id)?;

        // ensure we have permissions
        self.nft_check_can_send(ctx.store, msg_sender, token_id, &approvals)?;

        let owner = TOKEN_OWNER.load(ctx.store, token_id)?;
        // remove old position.owner
        OWNER_TO_TOKEN_IDS.remove(ctx.store, (owner, token_id.to_string()));
        // add to new position owner
        OWNER_TO_TOKEN_IDS.save(ctx.store, (recipient.clone(), token_id.to_string()), &1)?;
        TOKEN_OWNER.save(ctx.store, token_id, recipient)?;

        //reset existing approvals
        APPROVALS.save(ctx.store, token_id, &Vec::new())?;

        ctx.response_mut().add_event(TransferEvent {
            recipient: recipient.clone(),
            token_id: token_id.to_string(),
        });

        Ok(())
    }

    /// returns true iff the sender can transfer ownership of the token
    fn nft_check_can_send(
        &self,
        store: &dyn Storage,
        msg_sender: &Addr,
        token_id: &str,
        approvals: &[Approval],
    ) -> Result<()> {
        let owner = TOKEN_OWNER.load(store, token_id)?;
        if owner == *msg_sender {
            return Ok(());
        }

        // any non-expired token approval can send
        if approvals
            .iter()
            .any(|apr| apr.spender == *msg_sender && !apr.is_expired(&self.env.block))
        {
            return Ok(());
        }

        // operator can send
        let op = OPERATORS.may_load(store, (owner, msg_sender.clone()))?;
        match op {
            Some(ex) => {
                if ex.is_expired(&self.env.block) {
                    bail!("expired");
                } else {
                    Ok(())
                }
            }
            None => {
                bail!("auth");
            }
        }
    }
    fn nft_increment_tokens(&self, ctx: &mut StateContext) -> Result<u64> {
        let val = self.nft_num_tokens(ctx.store)?.count + 1;
        TOKEN_COUNT.save(ctx.store, &val)?;
        Ok(val)
    }
    fn _nft_decrement_tokens(&self, ctx: &mut StateContext) -> Result<u64> {
        let val = self.nft_num_tokens(ctx.store)?.count - 1;
        TOKEN_COUNT.save(ctx.store, &val)?;
        Ok(val)
    }
}

fn filter_approvals(
    block: &BlockInfo,
    approvals: &[Approval],
    include_expired: bool,
) -> Vec<Approval> {
    approvals
        .iter()
        .filter(|apr| include_expired || !apr.is_expired(block))
        .cloned()
        .collect()
}