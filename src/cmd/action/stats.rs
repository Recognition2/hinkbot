use std::time::Duration;

use diesel::result::Error as DieselError;
use failure::{
    Error as FailureError,
    SyncFailure,
};
use futures::{Future, future::err};
use telegram_bot::{
    Error as TelegramError,
    prelude::*,
    types::{Message, ParseMode},
};

use state::State;
use super::Action;

/// The action command name.
const CMD: &'static str = "stats";

/// Whether the action is hidden.
const HIDDEN: bool = false;

/// The action help.
const HELP: &'static str = "Display message stats";

pub struct Stats;

impl Stats {
    pub fn new() -> Self {
        Stats
    }
}

impl Action for Stats {
    fn cmd(&self) -> &'static str {
        CMD
    }

    fn hidden(&self) -> bool {
        HIDDEN
    }

    fn help(&self) -> &'static str {
        HELP
    }

    fn invoke(&self, state: &State, msg: &Message)
        -> Box<Future<Item = (), Error = FailureError>>
    {
        // Fetch the chat message stats
        let stats = match state
            .stats()
            .fetch_chat_stats(state.db(), msg.chat.id())
        {
            Ok(stats) => stats,
            Err(e) => return Box::new(err(e.into())),
        };

        // Build the chat message
        let mut response = String::from("*Messages (edits):*\n");

        // Append the user totals
        if !stats.users().is_empty() {
            let totals: Vec<String> = stats.users()
                .iter()
                .enumerate()
                .map(|(i, (user, messages, edits))| if *edits > 0 {
                    format!("_{}._ {}: _{} ({})_", i + 1, user, messages, edits)
                } else {
                    format!("_{}._ {}: _{}_", i + 1, user, messages)
                })
                .collect();
            response += &totals.join("\n");
        } else {
            response += "_No user stats yet_";
        }

        // Add other stats
        response += "\n\n*Other stats:*";
        response += &format!(
            "\nTotal: _{} ({})_",
            stats.total_messages(),
            stats.total_edits(),
        );
        if let Some(since) = stats.since() {
            response += &format!("\nSince: `{}`", since);
        }

        // Build a message future for sending the response
        // TODO: make this time configurable
        let future = state.telegram_client()
            .send_timeout(
                msg.text_reply(response)
                    .parse_mode(ParseMode::Markdown),
                Duration::from_secs(10),
            )
            .map(|_| ())
            .map_err(|err| Error::Respond(SyncFailure::new(err)))
            .from_err();

        Box::new(future)
    }
}

/// A stats action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while fetching chat stats from the database.
    #[fail(display = "failed to fetch message stats from database")]
    FetchStats(#[cause] DieselError),

    /// An error occurred while sending a response message to the user.
    #[fail(display = "failed to send response message")]
    Respond(#[cause] SyncFailure<TelegramError>),
}

impl From<DieselError> for Error {
    fn from(err: DieselError) -> Error {
        Error::FetchStats(err)
    }
}
