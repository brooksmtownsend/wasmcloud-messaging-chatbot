//! Discord specific logic using the [serenity] crate.

use std::{collections::HashMap, sync::Arc};

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::{Context as SerenityContext, *},
};
use wasmcloud_provider_sdk::get_connection;
use wit_bindgen_wrpc::tracing::{debug, error};

use crate::wasmcloud::messaging::types;

#[derive(Clone)]
pub struct DiscordHandler {
    pub component_id: String,
    pub handlers: Arc<RwLock<HashMap<String, (SerenityContext, Message)>>>,
}

impl DiscordHandler {
    pub fn new(component_id: &str) -> Self {
        Self {
            component_id: component_id.to_string(),
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: SerenityContext, msg: Message) {
        let broker_msg = types::BrokerMessage {
            subject: msg.channel_id.to_string(),
            reply_to: Some(msg.id.to_string()),
            body: msg.content.as_bytes().to_vec(),
        };

        self.handlers
            .write()
            .await
            .insert(msg.id.to_string(), (ctx, msg));
        match crate::wasmcloud::messaging::handler::handle_message(
            &get_connection().get_wrpc_client(&self.component_id),
            &broker_msg,
        )
        .await
        {
            Ok(Ok(_)) => (),
            Ok(Err(e)) => error!("Library error handling message: {:?}", e),
            Err(e) => error!("Protocol error handling message: {:?}", e),
        }
    }

    async fn ready(&self, _: SerenityContext, ready: Ready) {
        debug!("{} is connected!", ready.user.name);
    }
}
