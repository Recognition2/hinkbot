extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate telegram_bot;
extern crate tokio_core;

mod app;
mod cmd;
mod msg;

use std::env;

use futures::{Future, Stream};
use futures::future::ok;
use tokio_core::reactor::Core;
use telegram_bot::*;

use msg::handler::Handler;

fn main() {
    // Build a future reactor
    let mut core = Core::new().unwrap();
    let core_handle = core.handle();

    // Retrieve the Telegram bot token, initiate the API client
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("env var TELEGRAM_BOT_TOKEN not set");
    let api = Api::configure(token).build(core.handle()).unwrap();

    // Build a future for handling all updates from Telegram
    let future = api.stream()
        // Log text messages
        .inspect(|update| {
            if let UpdateKind::Message(message) = &update.kind {
                if let MessageKind::Text {ref data, ..} = message.kind {
                    println!(
                        "MSG <{}>@{}: {}",
                        &message.from.first_name,
                        &message.chat.id(),
                        data,
                    );
                }
            }
        })

        // TODO: do not mask Telegram errors here
        .map_err(|e| {
            eprintln!("ERROR: {:?}", e)
        })

        // Route new messages through the message handler, drop other updates
        .for_each(|update| {
            if let UpdateKind::Message(message) = update.kind {
                // Build a future to process the message, spawn it on the reactor
                core_handle.spawn(Handler::handle(message, &api));
            }

            ok(())
        });

    // Run the bot handling future in the reactor
    core.run(future).unwrap();
}
