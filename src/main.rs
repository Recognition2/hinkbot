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
mod state;
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
use telegram_bot::types::UpdateKind;

use msg::handler::Handler;
use util::handle_msg_error;
use state::State;

fn main() {
    // Load the environment variables file
    dotenv().ok();

    // Build a future reactor
    let mut core = Core::new().unwrap();
    let core_handle = core.handle();

    // Initialize the global state
    let state = State::init(core.handle());

    // Connect to the database
    let database_url = env::var("DATABASE_URL")
        .expect("env var DATABASE_URL not set");
    let db = MysqlConnection::establish(&database_url)
        .expect(&format!("Failed to connect to database on {}", database_url));

    // Build a future for handling all updates from Telegram
    let future = state.telegram_client().stream()
        // Route new messages through the message handler, drop other updates
        .for_each(|update| {
            // Process messages
            if let UpdateKind::Message(message) = update.kind {
                // Clone the state to get ownership
                let state = state.clone();

                // Build the message handling future, handle any errors
                let msg_handler = Handler::handle(
                        &state,
                        message.clone(),
                    )
                    .or_else(|err| handle_msg_error(state, message, err)
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
