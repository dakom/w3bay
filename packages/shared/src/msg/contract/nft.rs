use cosmwasm_schema::{QueryResponses, cw_serde};
use cosmwasm_std::{Addr, Binary, BlockInfo, IbcChannel};
use cw_utils::Expiration;

#[cw_serde]
pub struct InstantiateMsg { }

#[cw_serde]
pub enum IbcExecuteMsg {
    Mint {
        owner: String,
        metadata: Metadata,
    }
}

/// Matches the CW721 standard.
#[cw_serde]
pub enum ExecuteMsg {
    /// Transfer is a base message to move a token to another account without triggering actions
    TransferNft {
        /// Recipient of the NFT
        recipient: String,
        /// represented as a `String` to match the NFT spec
        token_id: String,
    },
    /// Send is a base message to transfer a token to a contract and trigger an action
    /// on the receiving contract.
    SendNft {
        /// Contract to receive the token 
        contract: String,
        /// represented as a `String` to match the NFT spec
        token_id: String,
        /// Message to execute on the contract
        msg: Binary,
    },
    /// Allows operator to transfer / send the token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    Approve {
        /// Address that is allowed to spend the NFT
        spender: String,
        /// represented as a `String` to match the NFT spec
        token_id: String,
        /// When the approval expires
        expires: Option<Expiration>,
    },
    /// Remove previously granted Approval
    Revoke {
        /// Address that is no longer allowed to spend the NFT
        spender: String,
        /// represented as a `String` to match the NFT spec
        token_id: String,
    },
    /// Allows operator to transfer / send any token from the owner's account.
    /// If expiration is set, then this allowance has a time/height limit
    ApproveAll {
        /// Address that is allowed to spend all NFTs by the sending wallet
        operator: String,
        /// When the approval expires
        expires: Option<Expiration>,
    },
    /// Remove previously granted ApproveAll permission
    RevokeAll {
        /// Address that is no longer allowed to spend all NFTs
        operator: String,
    },

    /// Burn will also remove the purchase on the Warehouse contract via IBC
    Burn {
        /// represented as a `String` to match the NFT spec
        token_id: String,
    }
}

/// Matches the CW721 standard.
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// * returns [OwnerOfResponse]
    ///
    /// Return the owner of the given token, error if token does not exist
    #[returns(OwnerOfResponse)]
    OwnerOf {
        /// represented as a `String` to match the NFT spec
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// * returns [ApprovalResponse]
    ///
    /// Return operator that can access all of the owner's tokens.
    #[returns(ApprovalResponse)]
    Approval {
        /// represented as a `String` to match the NFT spec
        token_id: String,
        /// Spender
        spender: String,
        /// Should we include expired approvals?
        include_expired: Option<bool>,
    },

    /// * returns [ApprovalsResponse]
    ///
    /// Return approvals that a token has
    #[returns(ApprovalsResponse)]
    Approvals {
        /// represented as a `String` to match the NFT spec
        token_id: String,
        /// Should we include expired approvals?
        include_expired: Option<bool>,
    },

    /// * returns [OperatorsResponse]
    ///
    /// List all operators that can access all of the owner's tokens
    #[returns(OperatorsResponse)]
    AllOperators {
        /// represented as a `String` to match the NFT spec
        owner: String,
        /// unset or false will filter out expired items, you must set to true to see them
        include_expired: Option<bool>,
        /// Last operator seen
        start_after: Option<String>,
        /// How many operators to return
        limit: Option<u32>,
    },

    /// * returns [NumTokensResponse]
    ///
    /// Total number of tokens issued
    #[returns(NumTokensResponse)]
    NumTokens {},

    /// * returns [NftContractInfo]
    ///
    /// Returns top-level metadata about the contract: `ContractInfoResponse`
    #[returns(NftContractInfo)]
    ContractInfo {},

    /// * returns [NftInfoResponse]
    ///
    /// Returns metadata for a given token
    /// the format is based on the *ERC721 Metadata JSON Schema*
    /// but directly from the contract: `NftInfoResponse`
    #[returns(NftInfoResponse)]
    NftInfo {
        /// Position ID, represented as a `String` to match the NFT spec
        token_id: String,
    },

    /// * returns [AllNftInfoResponse]
    ///
    /// Returns the result of both `NftInfo` and `OwnerOf` as one query as an optimization
    /// for clients: `AllNftInfo`
    #[returns(AllNftInfoResponse)]
    AllNftInfo {
        /// Position ID, represented as a `String` to match the NFT spec
        token_id: String,
        /// unset or false will filter out expired approvals, you must set to true to see them
        include_expired: Option<bool>,
    },

    /// * returns [TokensResponse]
    ///
    /// Returns all tokens owned by the given address, [] if unset.
    #[returns(TokensResponse)]
    Tokens {
        /// Owner to enumerate over
        owner: String,
        /// Last position ID seen
        start_after: Option<String>,
        /// Number of positions to return
        limit: Option<u32>,
    },

    /// * returns [TokensResponse]
    ///
    /// Requires pagination. Lists all token_ids controlled by the contract.
    #[returns(TokensResponse)]
    AllTokens {
        /// Last position ID seen
        start_after: Option<String>,
        /// Number of positions to return
        limit: Option<u32>,
    },

    /************ Not part of CW721 standard ************/
    /// * returns [cosmwasm_std::IbcChannel]
    #[returns(cosmwasm_std::IbcChannel)]
    IbcChannel {},

    /// * returns Vec<[NftInfoAndIdResponse]>
    ///
    /// Returns metadata for the given tokens
    /// the format is based on the *ERC721 Metadata JSON Schema*
    /// but directly from the contract: `NftInfoResponse`
    #[returns(Vec<NftInfoAndIdResponse>)]
    NftInfos {
        /// Token ids
        token_ids: Vec<String>,
    },

    /// Get general information about the contract 
    #[returns(InfoResp)]
    Info { }
}

#[cw_serde]
pub struct InfoResp {
    pub ibc_channel: Option<IbcChannel>
}

/// Placeholder migration message
#[cw_serde]
pub struct MigrateMsg {}

/// Response for [QueryMsg::OwnerOf]
#[cw_serde]
pub struct OwnerOfResponse {
    /// Owner of the token
    pub owner: Addr,
    /// If set this address is approved to transfer/send the token as well
    pub approvals: Vec<Approval>,
}

/// Response for [QueryMsg::Approval]
#[cw_serde]
pub struct ApprovalResponse {
    /// Approval information
    pub approval: Approval,
}

/// Response for [QueryMsg::Approvals]
#[cw_serde]
pub struct ApprovalsResponse {
    /// Approval information
    pub approvals: Vec<Approval>,
}

/// Response for [QueryMsg::Operators]
#[cw_serde]
pub struct OperatorsResponse {
    /// Operator approval information
    pub operators: Vec<Approval>,
}

/// Response for [QueryMsg::NumTokens]
#[cw_serde]
pub struct NumTokensResponse {
    /// Total number of tokens in the protocol
    pub count: u64,
}

/// Response for [QueryMsg::ContractInfo]
#[cw_serde]
pub struct NftContractInfo {
    /// Name of this contract
    pub name: String,
    /// Ticker symbol for this contract
    pub symbol: String,
}

/// Response for [QueryMsg::NftInfo]
#[cw_serde]
pub struct NftInfoResponse {
    /// You can add any custom metadata here when you extend cw721-base
    pub extension: Metadata,
}

/// Response for [QueryMsg::NftInfos]
#[cw_serde]
pub struct NftInfoAndIdResponse {
    pub token_id: String,
    /// You can add any custom metadata here when you extend cw721-base
    pub extension: Metadata,
}

/// Response for [QueryMsg::AllNftInfo]
#[cw_serde]
pub struct AllNftInfoResponse {
    /// Who can transfer the token
    pub access: OwnerOfResponse,
    /// Data on the token itself,
    pub info: NftInfoResponse,
}

/// Response for [QueryMsg::Tokens]
#[cw_serde]
pub struct TokensResponse {
    /// Contains all token_ids in lexicographical ordering
    /// If there are more than `limit`, use `start_from` in future queries
    /// to achieve pagination.
    pub tokens: Vec<String>,
}

/// copied/adapted from the cw721-base reference
#[cw_serde]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: Addr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

impl Approval {
    /// Is the given approval expired at the given block?
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        self.expires.is_expired(block)
    }
}

/// copied/adapted from the cw721-base reference
#[cw_serde]
pub struct FullTokenInfo {
    /// The owner of the newly minted NFT
    pub owner: Addr,
    /// Approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// metadata, as per spec
    pub extension: Metadata,
}

/// NFT standard metadata
#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
}

/// NFT-standard traits, used to express information on the position
#[cw_serde]
#[derive(Eq, Default)]
pub struct Trait {
    /// Unused
    pub display_type: Option<String>,
    /// The type of data contained in this trait.
    pub trait_type: String,
    /// The value for the given trait type.
    pub value: String,
}

/// Cw721ReceiveMsg should be de/serialized under `Receive()` variant in a ExecuteMsg
#[cw_serde]
pub struct Cw721ReceiveMsg {
    /// Sender of the NFT
    pub sender: String,
    /// Position ID transferred
    pub token_id: String,
    /// Binary message for the receiving contract to execute.
    pub msg: Binary,
}

pub mod events {
    use cosmwasm_std::{Addr, Event};
    use cw_utils::Expiration;
    use crate::event::CosmwasmEventExt;

    /// New NFT was minted
    #[derive(Debug)]
    pub struct MintEvent {
        pub token_id: String,
        pub owner: Addr,
    }

    impl From<MintEvent> for Event {
        fn from(src: MintEvent) -> Self {
            Event::new("mint").add_attributes(vec![
                ("token_id", src.token_id.to_string()),
                ("owner", src.owner.to_string()),
            ])
        }
    }

    impl TryFrom<Event> for MintEvent {
        type Error = anyhow::Error;

        fn try_from(evt: Event) -> anyhow::Result<Self> {
            Ok(MintEvent {
                token_id: evt.string_attr("token_id")?,
                owner: evt.unchecked_addr_attr("owner")?,
            })
        }
    }

    #[derive(Debug)]
    pub struct BurnEvent {
        pub token_id: String,
    }

    impl From<BurnEvent> for Event {
        fn from(src: BurnEvent) -> Self {
            Event::new("burn").add_attributes(vec![("token_id", src.token_id)])
        }
    }

    impl TryFrom<Event> for BurnEvent {
        type Error = anyhow::Error;

        fn try_from(evt: Event) -> anyhow::Result<Self> {
            Ok(BurnEvent {
                token_id: evt.string_attr("token_id")?,
            })
        }
    }

    // converting expiration back into an event is painful
    // so these are just unidirectional for now

    /// Approval was granted
    #[derive(Debug)]
    pub struct ApprovalEvent {
        /// Position
        pub token_id: String,
        /// Who can spend it
        pub spender: Addr,
        /// When it expires
        pub expires: Expiration,
    }

    impl From<ApprovalEvent> for Event {
        fn from(src: ApprovalEvent) -> Self {
            Event::new("approval").add_attributes(vec![
                ("token_id", src.token_id.to_string()),
                ("spender", src.spender.to_string()),
                ("expires", src.expires.to_string()),
            ])
        }
    }

    /// Approval was revoked
    #[derive(Debug)]
    pub struct RevokeEvent {
        /// Position ID
        pub token_id: String,
        /// Whose spend permissions were revoked
        pub spender: Addr,
    }

    impl From<RevokeEvent> for Event {
        fn from(src: RevokeEvent) -> Self {
            Event::new("revoke").add_attributes(vec![
                ("token_id", src.token_id.to_string()),
                ("spender", src.spender.to_string()),
            ])
        }
    }

    /// An operator was granted spend permissions on all positions for a wallet
    #[derive(Debug)]
    pub struct ApproveAllEvent {
        /// Who is the operator
        pub operator: Addr,
        /// When does the permission expire
        pub expires: Expiration,
    }

    impl From<ApproveAllEvent> for Event {
        fn from(src: ApproveAllEvent) -> Self {
            Event::new("approve-all").add_attributes(vec![
                ("operator", src.operator.to_string()),
                ("expires", src.expires.to_string()),
            ])
        }
    }

    /// Revoke all permissions for an operator
    #[derive(Debug)]
    pub struct RevokeAllEvent {
        /// Operator to revoke
        pub operator: Addr,
    }

    impl From<RevokeAllEvent> for Event {
        fn from(src: RevokeAllEvent) -> Self {
            Event::new("revoke-all").add_attributes(vec![("operator", src.operator.to_string())])
        }
    }

    /// NFT was transferred
    #[derive(Debug)]
    pub struct TransferEvent {
        /// New owner
        pub recipient: Addr,
        /// Position ID
        pub token_id: String,
    }

    impl From<TransferEvent> for Event {
        fn from(src: TransferEvent) -> Self {
            Event::new("transfer").add_attributes(vec![
                ("recipient", src.recipient.to_string()),
                ("token_id", src.token_id),
            ])
        }
    }
}