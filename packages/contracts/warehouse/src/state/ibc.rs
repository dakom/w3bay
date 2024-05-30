use cosmwasm_std::{
    from_binary, from_json, to_json_binary, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcTimeout, Storage
};
use cw_storage_plus::Item;
use shared::{ibc::{
    event::{IbcChannelCloseEvent, IbcChannelConnectEvent},
    validate_ibc_channel_order_and_version, TIMEOUT_SECONDS,
}, msg::{contract::{nft::{Metadata, Trait}, warehouse::{IbcExecuteMsg, PurchaseError}}, product}};
use anyhow::Result;

use super::{State, StateContext};

const IBC_PAYMENT_CHANNEL: Item<IbcChannel> = Item::new("ibc-payment-channel");
const IBC_NFT_CHANNEL: Item<IbcChannel> = Item::new("ibc-nft-channel");

pub enum IbcChannelKind {
    Payment,
    Nft
}

impl State<'_> {
    pub fn get_ibc_channel(&self, store: &dyn Storage, kind: IbcChannelKind) -> Result<IbcChannel> {
        match kind {
            IbcChannelKind::Payment => IBC_PAYMENT_CHANNEL.load(store).map_err(|err| err.into()),
            IbcChannelKind::Nft => IBC_NFT_CHANNEL.load(store).map_err(|err| err.into())
        }
    }

    pub fn handle_ibc_channel_open(&self, msg: IbcChannelOpenMsg) -> Result<()> {
        validate_ibc_channel_order_and_version(msg.channel(), msg.counterparty_version())?;
        Ok(())
    }

    pub fn handle_ibc_channel_connect(
        &self,
        ctx: &mut StateContext,
        msg: IbcChannelConnectMsg,
    ) -> Result<()> {
        let channel = msg.channel();

        validate_ibc_channel_order_and_version(channel, msg.counterparty_version())?;

        match channel.version.as_str() {
            "warehouse-payment-001" => {
                IBC_PAYMENT_CHANNEL.save(ctx.store, channel)?;
            },
            "warehouse-nft-001" => {
                IBC_NFT_CHANNEL.save(ctx.store, channel)?;
            },
            _ => anyhow::bail!("unsupported channel version: {}", channel.version)
        }


        ctx.response_mut()
            .add_event(IbcChannelConnectEvent { channel });

        Ok(())
    }

    pub fn handle_ibc_channel_close(
        &self,
        ctx: &mut StateContext,
        msg: IbcChannelCloseMsg,
    ) -> Result<()> {
        let channel = msg.channel();

        ctx.response_mut()
            .add_event(IbcChannelCloseEvent { channel });
        Ok(())
    }

    pub fn handle_ibc_packet_receive(
        &self,
        ctx: &mut StateContext,
        msg: IbcPacketReceiveMsg,
    ) -> Result<()> {
        from_json(&msg.packet.data)
            .map_err(|err| err.into())
            .and_then(|msg| {
                match msg {
                    IbcExecuteMsg::Purchase{ owner, spender, product_id, quantity, fees} => {
                        match self.make_purchase(ctx, spender.clone(), product_id, quantity, fees.clone()) {
                            Ok(purchase_id) => {
                                let mut metadata = Metadata::default();
                                metadata.name = Some(format!("Purchase #{}", purchase_id));
                                metadata.description = Some(format!("Product #{}", product_id));
                                metadata.attributes = Some(vec![
                                    Trait {
                                        display_type: None,
                                        trait_type: "product-id".to_string(),
                                        value: product_id.to_string() 
                                    },
                                    Trait {
                                        display_type: None,
                                        trait_type: "purchase-id".to_string(),
                                        value: purchase_id.to_string() 
                                    },
                                    Trait {
                                        display_type: None,
                                        trait_type: "quantity".to_string(),
                                        value: quantity.to_string() 
                                    },
                                ]);
                        
                                let msg = shared::msg::contract::nft::IbcExecuteMsg::Mint { owner, metadata };
                        
                                // outbound IBC message, where packet is then received on other chain
                                let channel_id = self
                                    .get_ibc_channel(ctx.store, IbcChannelKind::Nft)?
                                    .endpoint
                                    .channel_id;
                        
                                ctx.response_mut().add_message(IbcMsg::SendPacket {
                                    channel_id,
                                    data: to_json_binary(&msg)?,
                                    // default timeout of two minutes.
                                    timeout: IbcTimeout::with_timestamp(self.env.block.time.plus_seconds(TIMEOUT_SECONDS)),
                                });

                                Ok(())
                            } 
                            Err(err) => {
                                Err(anyhow::Error::new(PurchaseError {
                                    owner,
                                    spender,
                                    product_id,
                                    quantity,
                                    fees,
                                    reason: err.to_string() 
                                }))
                            }
                        }
                    },

                    IbcExecuteMsg::RemovePurchase { id } => {
                        match self.remove_purchase(ctx, id) {
                            Ok(_) => Ok(()),
                            Err(err) => Err(err.into())
                        }
                    }
                    
                }
            })
    }

    pub fn handle_ibc_packet_ack(&self, _ack: IbcPacketAckMsg) -> Result<()> {
        // Nothing to do here. We don't keep any state about the other
        // chain, just deliver messages so nothing to update.
        //
        // If we did care about how the other chain received our message
        // we could deserialize the data field into an `Ack` and inspect
        // it.
        Ok(())
    }

    pub fn handle_ibc_packet_timeout(&self, _msg: IbcPacketTimeoutMsg) -> Result<()> {
        // As with ack above, nothing to do here. If we cared about
        // keeping track of state between the two chains then we'd want to
        // respond to this likely as it means that the packet in question
        // isn't going anywhere.
        Ok(())
    }
}