use std::{collections::HashMap, sync::Arc};

use anyhow::Context as _;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use wasmcloud_provider_sdk::{get_connection, run_provider, Context, Provider};
use wit_bindgen_wrpc::tracing::{debug, error, warn};

mod discord;
use discord::DiscordHandler;

wit_bindgen_wrpc::generate!();

use crate::wasmcloud::messaging::types;

#[derive(Default, Clone)]
struct DiscordProvider {
    /// A map of component ID to [DiscordHandler] which contains the Serenity client and message handlers
    handlers: Arc<RwLock<HashMap<String, DiscordHandler>>>,
    /// A map of component ID to the task handle for the client
    client_tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl DiscordProvider {
    fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            client_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Provider for DiscordProvider {
    async fn receive_link_config_as_source(
        &self,
        config: wasmcloud_provider_sdk::LinkConfig<'_>,
    ) -> Result<(), anyhow::Error> {
        debug!("receiving link configuration as source");
        let source_config: case_insensitive_hashmap::CaseInsensitiveHashMap<String> =
            case_insensitive_hashmap::CaseInsensitiveHashMap::from_iter(
                config
                    .config
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string())),
            );

        if let Some(token) = source_config.get("token") {
            let handler = DiscordHandler::new(&config.target_id);
            let mut client =
                serenity::Client::builder(token, serenity::all::GatewayIntents::non_privileged())
                    .event_handler(handler.clone())
                    .await
                    .context("failed to create Discord client")?;
            let task = tokio::spawn(async move {
                debug!("handling client start in task");
                if let Err(why) = client.start().await {
                    error!("Client error: {:?}", why);
                }
            });

            self.handlers
                .write()
                .await
                .insert(config.target_id.to_string(), handler);
            self.client_tasks
                .write()
                .await
                .insert(config.target_id.to_string(), task);

            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "token is required for discord authentication"
            ))
        }
    }

    async fn delete_link(&self, component_id: &str) -> Result<(), anyhow::Error> {
        debug!(component_id, "deleting link");
        let _ = self.handlers.write().await.remove(component_id);

        Ok(())
    }
}

impl exports::wasmcloud::messaging::consumer::Handler<Option<Context>> for DiscordProvider {
    async fn request(
        &self,
        _: Option<Context>,
        _: String,
        _: Vec<u8>,
        _: u32,
    ) -> Result<Result<types::BrokerMessage, String>, anyhow::Error> {
        error!("request not implemented");
        anyhow::bail!("request not implemented")
    }

    async fn publish(
        &self,
        ctx: Option<Context>,
        msg: types::BrokerMessage,
    ) -> Result<Result<(), String>, anyhow::Error> {
        debug!("component publishing message as bot");
        let Some(Some(component_id)) = ctx.as_ref().map(|ctx| ctx.component.as_ref()) else {
            error!("missing component ID");
            return Ok(Err("missing component ID".to_string()));
        };

        let handlers = self.handlers.read().await;
        let component_handler = handlers.get(component_id).unwrap();

        let Some((ctx, original_msg)) = component_handler.message(&msg.subject).await else {
            error!(
                message_id = msg.subject,
                "component published message with unknown ID"
            );
            return Ok(Err("message not found for specified ID".to_string()));
        };

        let message_text = String::from_utf8_lossy(&msg.body);
        // This shouldn't really ever happen, but just in case this warning will help identify cases
        // where we need better string handling of incoming messages
        if message_text.contains(char::REPLACEMENT_CHARACTER) {
            warn!("message body is not valid UTF-8, may contain invalid characters");
        }
        original_msg
            .channel_id
            .say(&ctx.http, message_text)
            .await
            .map(|_| Ok(()))
            .map_err(|e| anyhow::anyhow!(e))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider = DiscordProvider::new();
    let shutdown = run_provider(provider.clone(), "messaging-discord-provider")
        .await
        .context("failed to run provider")?;
    let connection = get_connection();
    serve(
        &connection.get_wrpc_client(connection.provider_key()),
        provider,
        shutdown,
    )
    .await
}
