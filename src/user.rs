use std::sync::{Arc, RwLock};

use drift::state::user::User;
use solana_sdk::pubkey::Pubkey;

use crate::{
    event_emitter::EventEmitter,
    utils::{decode, get_ws_url},
    websocket_account_subscriber::{AccountUpdate, WebsocketAccountSubscriber},
    AccountProvider, DataAndSlot, DriftClient, SdkResult,
};

#[derive(Clone)]
pub struct DriftUser {
    pub pubkey: Pubkey,
    subscription: WebsocketAccountSubscriber,
    data_and_slot: Arc<RwLock<DataAndSlot<User>>>,
    pub sub_account: u16,
}

impl DriftUser {
    pub const SUBSCRIPTION_ID: &'static str = "user";

    pub async fn new<T: AccountProvider>(
        pubkey: Pubkey,
        drift_client: &DriftClient<T>,
        sub_account: u16,
    ) -> SdkResult<Self> {
        let subscription = WebsocketAccountSubscriber::new(
            DriftUser::SUBSCRIPTION_ID,
            get_ws_url(&drift_client.inner().url()).expect("valid url"),
            pubkey,
            drift_client.inner().commitment(),
            EventEmitter::new(),
        );

        let user = drift_client.get_user_account(&pubkey).await?;
        let data_and_slot = Arc::new(RwLock::new(DataAndSlot {
            data: user,
            slot: 0,
        }));

        Ok(Self {
            pubkey,
            subscription,
            data_and_slot,
            sub_account,
        })
    }

    pub async fn subscribe(&mut self) -> SdkResult<()> {
        let current_data_and_slot = self.data_and_slot.clone();
        self.subscription
            .event_emitter
            .subscribe(DriftUser::SUBSCRIPTION_ID, move |event| {
                if let Some(update) = event.as_any().downcast_ref::<AccountUpdate>() {
                    let new_data =
                        decode::<User>(update.data.data.clone()).expect("valid user data");
                    let slot = update.slot;
                    let mut data_and_slot = current_data_and_slot.write().unwrap();
                    *data_and_slot = DataAndSlot {
                        data: new_data,
                        slot,
                    };
                }
            });
        self.subscription.subscribe().await?;
        Ok(())
    }

    pub fn get_user_account_and_slot(&self) -> DataAndSlot<User> {
        let reader = self.data_and_slot.read().expect("reader");
        reader.clone()
    }

    pub fn get_user_account(&self) -> User {
        self.get_user_account_and_slot().data
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::Context;
    use crate::RpcAccountProvider;
    use solana_sdk::signature::Keypair;

    const DEVNET_ENDPOINT: &str = "https://api.devnet.solana.com";

    #[tokio::test]
    // #[cfg(feature = "rpc_tests")]
    async fn test_user_subscribe() {
        let url = DEVNET_ENDPOINT;
        let client = DriftClient::new(
            Context::DevNet,
            RpcAccountProvider::new(&url),
            Keypair::new().into(),
        )
        .await
        .unwrap();

        let pubkey = Pubkey::from_str("9JtczxrJjPM4J1xooxr2rFXmRivarb4BwjNiBgXDwe2p").unwrap();
        let mut user = DriftUser::new(pubkey, &client, 0).await.unwrap();
        user.subscribe().await.unwrap();

        let mut count = 0;
        loop {
            if count > 5 {
                break;
            }

            let data_and_slot = user.get_user_account_and_slot();
            dbg!(data_and_slot.slot);

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            count += 1;
        }
    }
}
