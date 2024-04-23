wit_bindgen::generate!();

use exports::wasmcloud::messaging::handler::Guest;
use wasi::logging::logging::*;
use wasmcloud::messaging::*;

struct BadJanet;

impl Guest for BadJanet {
    fn handle_message(msg: types::BrokerMessage) -> Result<(), String> {
        let content = String::from_utf8_lossy(&msg.body);

        if content.contains("ping") && msg.reply_to.is_some() {
            let _ = consumer::publish(&types::BrokerMessage {
                subject: msg.reply_to.clone().unwrap(),
                reply_to: None,
                body: b"Pong, ya fat dingus".to_vec(),
            });
            consumer::publish(&types::BrokerMessage {
                subject: msg.reply_to.unwrap(),
                reply_to: None,
                body: b"Now go back to the good place".to_vec(),
            })
        } else {
            log(
                Level::Debug,
                "",
                "Received message that didn't contain 'ping' in the body",
            );
            Ok(())
        }
    }
}

export!(BadJanet);
