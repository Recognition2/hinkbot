use diesel::result::Error as DieselError;
use failure::{
    Error as FailureError,
    SyncFailure,
};
use futures::{
    Future,
    future::{err, ok},
};
use telegram_bot::{
    Error as TelegramError,
    prelude::*,
    types::{Message, MessageChat, MessageKind, ParseMode},
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
        // Do not respond in non-private chats
        match &msg.kind {
            MessageKind::Text { ..  } => match &msg.chat {
                MessageChat::Private(..) => {},
                _ => return Box::new(ok(())),
            },
            _ => {},
        }

        // Fetch the chat message stats
        let stats = match state
            .stats()
            .fetch_chat_stats(state.db(), msg.chat.id(), Some(msg.from.id))
        {
            Ok(stats) => stats,
            Err(e) => return Box::new(err(e.into())),
        };

        // Build the chat message
        let mut response = String::from("*Messages (edits):*\n");

        // Append the user totals
        let totals: Vec<String> = stats.users()
            .iter()
            .map(|(user, _, username, messages, edits)| match username {
                Some(username) if !username.is_empty() => 
                    (format!("[{}](https://t.me/{})", user, username), messages, edits),
                _ => (user.to_owned(), messages, edits),
            })
            .enumerate()
            .map(|(i, (name, messages, edits))| if *edits > 0 {
                format!("{}. {}: _{} ({})_", i + 1, name, messages, edits)
            } else {
                format!("{}. {}: _{}_", i + 1, name, messages)
            })
            .collect();
        response += &totals.join("\n");

        // Append the user specifics if available
        if let Some(specific) = stats.specific() {
            response += "\n\n*Your messages (edits):*\n";
            let specific: Vec<String> = specific
                .iter()
                .map(|(kind, messages, edits)| if *edits > 0 {
                    format!("{}s: _{} ({})_", ucfirst(kind.name()), messages, edits)
                } else {
                    format!("{}s: _{}_", ucfirst(kind.name()), messages)
                })
                .collect();
            response += &specific.join("\n");
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
        let future = state
            .telegram_send(
                msg.text_reply(response)
                    .parse_mode(ParseMode::Markdown)
                    .disable_preview(),
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

/// Uppercase the first character.
fn ucfirst(string: &str) -> String {
    string.chars()
        .enumerate()
        .filter_map(|(i, c)| if i == 0 {
                c.to_uppercase().next()
            } else {
                Some(c)
            })
        .collect()
}
