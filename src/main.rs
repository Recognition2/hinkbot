extern crate chan_signal;
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
mod stats;
mod util;

use std::process::exit;
use std::thread;
use std::time::Duration;

use chan_signal::Signal;
use dotenv::dotenv;
use futures::{
    Future,
    future::{Executor, ok},
    Stream,
};
use tokio_core::reactor::{Core, Handle, Interval};
use telegram_bot::types::UpdateKind;

use msg::handler::Handler;
use util::handle_msg_error;
use state::State;

/// The application entrypoint.
fn main() {
    // Register signals to be identified with
    let signal = chan_signal::notify(&[Signal::INT, Signal::TERM]);

    // Load the environment variables file
    dotenv().ok();

    // Start the reactor
    thread::spawn(start_reactor);

    // Receive a signal to quit
    if let Some(signal) = signal.recv() {
        eprintln!("Received signal: {:?}", signal);
        // TODO: flush all stats to the database before quitting
        exit(0);
    } else {
        eprintln!("Failed to recieve signal");
        exit(1);
    }
}

/// Start the actual reactor, which will be the event loop for the Telegram API client and
/// additional processes.
fn start_reactor() {
    // Build a future reactor
    let core = Core::new().unwrap();

    // Initialize the global state
    let state = State::init(core.handle());

    // Build a stats flusher and Telegram API updates handler future
    let stats_flusher = build_stats_flusher(state.clone(), core.handle());
    let telegram = build_telegram_handler(state.clone(), core.handle());

    // Run the bot handling future in the reactor
    core.execute(
        telegram.join(stats_flusher).map(|_| ()),
    ).unwrap();
}

/// Build a future for handling Telegram API updates.
fn build_telegram_handler(state: State, handle: Handle)
    -> impl Future<Item = (), Error = ()>
{
    state.telegram_client()
        .stream()
        .for_each(move |update| {
            // Clone the state to get ownership
            let state = state.clone();

            // Process messages
            match update.kind {
                UpdateKind::Message(message) => {
                    // Update the message stats
                    state.stats().increase_message_stats(&message, 1, 0);

                    // Build the message handling future, handle any errors
                    let msg_handler = Handler::handle(
                            &state,
                            message.clone(),
                        )
                        .or_else(|err| handle_msg_error(state, message, err)
                            .or_else(|err| {
                                eprintln!(
                                    "ERR: failed to handle error while handling message: {:?}",
                                    err,
                                );
                                ok(())
                            })
                        );

                    // Spawn the message handler future on the reactor
                    handle.spawn(msg_handler);
                },
                UpdateKind::EditedMessage(message) =>
                    state.stats().increase_message_stats(&message, 0, 1),
                _ => {},
            }

            ok(())
        })
        .map_err(|err| {
            eprintln!("ERR: Telegram API updates loop error, ignoring: {}", err);
            ()
        })
}

/// Build a future for handling Telegram API updates.
// TODO: make the interval time configurable
fn build_stats_flusher(state: State, handle: Handle) -> impl Future<Item = (), Error = ()> {
    Interval::new(
            Duration::from_secs(3),
            &handle,
        )
        .expect("failed to build stats flushing interval future")
        .for_each(move |_| {
            println!("FLUSH");
            state.stats().flush(state.db());
            Ok(())
        })
        .map_err(|_| ())
}
