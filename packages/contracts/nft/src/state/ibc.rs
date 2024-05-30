use cosmwasm_std::{
    from_binary, from_json, IbcChannel, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, Storage
};
use cw_storage_plus::Item;
use shared::{ibc::{
    event::{IbcChannelCloseEvent, IbcChannelConnectEvent},
    validate_ibc_channel_order_and_version,
}, msg::contract::nft::IbcExecuteMsg};
use anyhow::Result;

use super::{State, StateContext};

const IBC_CHANNEL: Item<IbcChannel> = Item::new("ibc-channel");

impl State<'_> {
    pub fn get_ibc_channel(&self, store: &dyn Storage) -> Result<IbcChannel> {
        IBC_CHANNEL.load(store).map_err(|err| err.into())
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

        IBC_CHANNEL.save(ctx.store, channel)?;

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
                    IbcExecuteMsg::Mint{owner, metadata } => {
                        self.nft_mint(ctx, self.api.addr_validate(&owner)?, metadata)
                    }
                }
            })
    }

    pub fn handle_ibc_packet_ack(&self, _ack: IbcPacketAckMsg) -> Result<()> {
        // TODO - decode the payload and see if we got an error, so we can refund users on failure
        Ok(())
    }

    pub fn handle_ibc_packet_timeout(&self, _msg: IbcPacketTimeoutMsg) -> Result<()> {
        // TODO - decode the payload and see if we got an error, so we can refund users on failure
        Ok(())
    }
}