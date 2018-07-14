extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate dotenv;
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
mod models;
mod msg;
mod schema;
mod util;

use std::env; 

use diesel::{
    prelude::*,
    mysql::MysqlConnection,
};
use dotenv::dotenv;
use futures::{
    Future,
    future::ok,
    Stream,
};
use tokio_core::reactor::Core;
use telegram_bot::{
    Api,
    types::UpdateKind,
};

use msg::handler::Handler;
use util::handle_msg_error;

fn main() {
    // Load the environment variables file
    dotenv().ok();

    // Build a future reactor
    let mut core = Core::new().unwrap();
    let core_handle = core.handle();

    // Retieve environment variables
    let token = env::var("TELEGRAM_BOT_TOKEN")
        .expect("env var TELEGRAM_BOT_TOKEN not set");
    let database_url = env::var("DATABASE_URL")
        .expect("env var DATABASE_URL not set");

    // Connect to the database
    let db = MysqlConnection::establish(&database_url)
        .expect(&format!("Failed to connect to database on {}", database_url));

    // Initiate the Telegram API client
    let api = Api::configure(token)
        .build(core.handle())
        .unwrap();

    // Build a future for handling all updates from Telegram
    let future = api.stream()
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
                    )
                    .or_else(|err| handle_msg_error(message, api, err)
                        .or_else(|err| {
                            println!(
                                "ERR: failed to handle error while handling message: {:?}",
                                err,
                            );
                            ok(())
                        })
                    );

                // Spawn the message handler future on the reactor
                core_handle.spawn(msg_handler);
            }

            ok(())
        });

    // Run the bot handling future in the reactor
    core.run(future).unwrap();
}
