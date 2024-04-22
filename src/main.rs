use std::{collections::HashMap, sync::Arc};

use anyhow::Context as _;
use futures::Future;
use tokio::sync::RwLock;

use wasmcloud_provider_sdk::{get_connection, run_provider, Context, Provider};
use wit_bindgen_wrpc::tracing::{debug, error, info};

mod discord;
use discord::DiscordHandler;

wit_bindgen_wrpc::generate!();

use crate::wasmcloud::messaging::types;

#[derive(Default, Clone)]
struct DiscordProvider {
    clients: Arc<RwLock<HashMap<String, DiscordHandler>>>,
}

impl DiscordProvider {
    fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Provider for DiscordProvider {
    async fn receive_link_config_as_source(
        &self,
        config: wasmcloud_provider_sdk::LinkConfig<'_>,
    ) -> Result<(), anyhow::Error> {
        debug!("receiving link config as source");
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
                    .expect("Err creating client");

            self.clients
                .write()
                .await
                .insert(config.target_id.to_string(), handler);
            tokio::spawn(async move {
                info!("Starting client in task");
                if let Err(why) = client.start().await {
                    error!("Client error: {:?}", why);
                }
            });

            Ok(())
        } else {
            Err(anyhow::anyhow!("token is required"))
        }
    }

    async fn delete_link(&self, component_id: &str) -> Result<(), anyhow::Error> {
        debug!(component_id, "deleting link");
        let _ = self.clients.write().await.remove(component_id);

        Ok(())
    }

    fn shutdown(&self) -> impl Future<Output = Result<(), anyhow::Error>> + Send {
        async { Ok(()) }
    }
}

impl exports::wasmcloud::messaging::consumer::Handler<Option<Context>> for DiscordProvider {
    async fn request(
        &self,
        _: std::option::Option<wasmcloud_provider_sdk::Context>,
        _: std::string::String,
        _: Vec<u8>,
        _: u32,
    ) -> Result<Result<types::BrokerMessage, String>, anyhow::Error> {
        error!("request not implemented");
        anyhow::bail!("request not implemented")
    }

    async fn publish(
        &self,
        ctx: std::option::Option<wasmcloud_provider_sdk::Context>,
        msg: types::BrokerMessage,
    ) -> Result<Result<(), String>, anyhow::Error> {
        info!("publish called");
        let ctx = ctx.ok_or_else(|| anyhow::anyhow!("missing context"))?;
        let clients = self.clients.read().await;

        let source = clients.get(&ctx.component.unwrap()).unwrap();
        let handlers = source.handlers.read().await;
        let handler = handlers.get(&msg.subject);
        let content = String::from_utf8_lossy(&msg.body);

        let (ctx, msg) = handler.unwrap();

        let _ = msg.channel_id.say(&ctx.http, content).await;

        Ok(Ok(()))
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
