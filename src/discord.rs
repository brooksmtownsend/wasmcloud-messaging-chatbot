//! Discord specific logic using the [serenity] crate.

use std::{collections::HashMap, sync::Arc};

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::{Context as SerenityContext, *},
};
use wasmcloud_provider_sdk::get_connection;
use wit_bindgen_wrpc::tracing::{error, info, trace};

use crate::wasmcloud::messaging::types;

#[derive(Clone)]
pub struct DiscordHandler {
    /// The component ID that this handler is associated with
    pub component_id: String,
    /// A map of message IDs to Serenity context and message, so a component can
    /// reply asynchronously to a message
    pub messages: Arc<RwLock<HashMap<String, (SerenityContext, Message)>>>,
}

impl DiscordHandler {
    pub fn new(component_id: &str) -> Self {
        Self {
            component_id: component_id.to_string(),
            messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn message(&self, message_id: &str) -> Option<(SerenityContext, Message)> {
        self.messages.read().await.get(message_id).cloned()
    }
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: SerenityContext, msg: Message) {
        let discord_msg = types::BrokerMessage {
            subject: msg.channel_id.to_string(),
            reply_to: Some(msg.id.to_string()),
            body: msg.content.as_bytes().to_vec(),
        };

        let msg_id = msg.id.to_string();
        self.messages
            .write()
            .await
            .insert(msg.id.to_string(), (ctx, msg));

        // Spawns a task to remove the message from the map after 30 seconds, which should be
        // plenty of time for the component to reply.
        let messages = self.messages.clone();
        tokio::spawn(async move {
            let _ = tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            messages.write().await.remove(&msg_id);
        });

        match crate::wasmcloud::messaging::handler::handle_message(
            &get_connection().get_wrpc_client(&self.component_id),
            &discord_msg,
        )
        .await
        {
            Ok(Ok(_)) => trace!("Component handled message successfully"),
            Ok(Err(e)) => error!(e, "Component failed to handle message"),
            Err(e) => error!(%e, "RPC error handling message"),
        }
    }

    async fn ready(&self, _: SerenityContext, ready: Ready) {
        info!(bot_name = ready.user.name, "Discord bot connected");
    }
}
