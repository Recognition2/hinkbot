extern crate chrono;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate humansize;
extern crate humantime;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate telegram_bot;
extern crate tokio_core;

mod app;
mod cmd;
mod executor;
mod msg;
mod util;

use std::env; 
use futures::{
    Future,
    future::ok,
    Stream,
};
use tokio_core::reactor::Core;
use telegram_bot::*;

use msg::handler::Handler;
use util::handle_msg_error;

fn main() {
    // Build a future reactor
    let mut core = Core::new().unwrap();
    let core_handle = core.handle();

    // Retrieve the Telegram bot token, initiate the API client
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("env var TELEGRAM_BOT_TOKEN not set");
    let api = Api::configure(token)
        .build(core.handle())
        .unwrap();

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

        // Route new messages through the message handler, drop other updates
        .for_each(|update| {
            // Process messages
            if let UpdateKind::Message(message) = update.kind {
                // Clone the API instance to get ownership
                // TODO: do not clone this API as it's probably not needed
                let api = api.clone();

                // Build the message handling future, handle any errors
                let msg_handler = Handler::handle(
                    message.clone(),
                    &api,
                ).then(|result| {
                    // Handle errors
                    if let Err(err) = result {
                        handle_msg_error(message, api, err);
                    }

                    ok(())
                });

                // Spawn the message handler future on the reactor
                core_handle.spawn(msg_handler);
            }

            ok(())
        });

    // Run the bot handling future in the reactor
    core.run(future).unwrap();
}
