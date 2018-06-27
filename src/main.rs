extern crate futures;
extern crate regex;
extern crate telegram_bot;
extern crate tokio_core;

use std::env;

use futures::Stream;
use regex::Regex;
use tokio_core::reactor::Core;
use telegram_bot::*;

fn main() {
    let mut core = Core::new().unwrap();

    let token = env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let api = Api::configure(token).build(core.handle()).unwrap();

    // Fetch new updates via long poll method
    let future = api.stream().for_each(|update| {

        // If the received update contains a new message...
        if let UpdateKind::Message(message) = update.kind {

            if let MessageKind::Text {ref data, ..} = message.kind {
                // Print received text message to stdout.
                println!("DM <{}>: {}", &message.from.first_name, data);

                let msg = data.trim();
                if msg.starts_with('/') {
                    let re = Regex::new(r"^/ping(@.*$|$)").expect("failed to build ping regex");
                    if re.is_match(msg) {
                        api.spawn(message.text_reply("Pong!"));
                    }

                    let re = Regex::new(r"^/genimi(@.*$|$)").expect("failed to build genimi regex");
                    if re.is_match(msg) {
                        api.spawn(message.text_reply("Genimi bot v0.0.1\nDeveloped by Tim Visee, https://timvisee.com/"));
                    }
                } else {
                    api.spawn(message.text_reply(
                        format!("Hi, {}!\nDirect messages are not supported yet.", &message.from.first_name)
                    ));
                }
            }
        }

        Ok(())
    });

    core.run(future).unwrap();
}
