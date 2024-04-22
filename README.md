# Messaging Discord Provider

This capability provider uses the Discord API via the [serenity](https://crates.io/crates/serenity) crate to handle messages sent to a Discord bot. This is an implementation of the [wasmcloud:messaging](https://github.com/wasmCloud/messaging) interface.

## Usage

Since this provider uses the standard `wasmcloud:messaging` interface, you can implement it entirely using the generated types from that interface. This is best used by implementing `wasmcloud:messaging/handler.handle-message` which will be called with each Discord message, and then whatever you would like to say in response should be done by calling `wasmcloud:messaging/consumer.publish` with the message's `reply_to`. For example, a simple ping/pong bot in Rust:

```rust
wit_bindgen::generate!();

use exports::wasmcloud::messaging::handler::Guest;
use wasi::logging::logging::*;
use wasmcloud::messaging::*;

struct PingPong;

impl Guest for PingPong {
    fn handle_message(msg: types::BrokerMessage) -> Result<(), String> {
        let content = String::from_utf8_lossy(&msg.body);
        if content.contains("ping") && msg.reply_to.is_some() {
            consumer::publish(&types::BrokerMessage {
                subject: msg.reply_to.unwrap(),
                reply_to: None,
                body: b"Pong".to_vec(),
            })
        }

        Ok(())
    }
}

export!(PingPong);
```

See the [good janet wadm.yaml](./good-janet/wadm.yaml) for an example of how to configure this application. You need to supply a `token` as `source_config` for the link from the Discord provider to your component to ensure the provider can connect to Discord.

## Discord

Make sure to follow the [Discord Developer Portal Getting Started](https://discord.com/developers/docs/quick-start/getting-started) guide

## Future

- This provider does not support registering slash commands, but that would be a great enhancement for the future.
- This provider does not support subscribing only to a specific set of channels, but that would also be a great enhancement.
