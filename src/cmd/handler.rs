use futures::{
    Future,
    future::ok,
};
use regex::Regex;
use telegram_bot::types::Message;

use state::State;
use super::action::Error as ActionError;
use super::action::ACTIONS;

lazy_static! {
    /// A regex for matching messages that contain a command.
    static ref CMD_REGEX: Regex = Regex::new(
        r"^/(?is)([A-Z0-9_]+)(@[A-Z0-9_]+)?(\s.*$|$)",
    ).expect("failed to compile CMD_REGEX");
}

/// The command handler.
pub struct Handler;

impl Handler {
    /// Handle the given command.
    pub fn handle(state: &State, cmd: &str, msg: Message)
        -> Box<Future<Item = (), Error = Error>>
    {
        // Invoke the proper action
        let action = ACTIONS.iter()
            .find(|a| a.is_cmd(cmd));
        if let Some(action) = action {
            // Build the action invocation future
            let action_future = action
                .invoke(&state, &msg)
                .map_err(move |err| ActionError::Invoke {
                    cause: err.compat(),
                    name: action.cmd().to_owned(),
                })
                .from_err();

            Box::new(action_future)
        } else {
            Box::new(ok(()))
        }
    }
}

/// Test wether the given message is recognized as a command.
///
/// The actual command name is returned if it is, `None` otherwise.
// TODO: if a target bot is given with `/cmd@bot`, ensure it's username is matching
pub fn matches_cmd(msg: &str) -> Option<&str> {
    if let Some(groups) = CMD_REGEX.captures(msg.trim()) {
        Some(groups.get(1).unwrap().as_str())
    } else {
        None
    }
}

/// A command handler error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while handling a command.
    #[fail(display = "failed to invoke command")]
    Cmd(#[cause] ActionError),
}

/// Convert command action errors to a command handler error.
impl From<ActionError> for Error {
    fn from(err: ActionError) -> Error {
        Error::Cmd(err)
    }
}
