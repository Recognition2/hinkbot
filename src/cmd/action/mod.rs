pub mod exec;
pub mod help;
pub mod id;
pub mod ping;
pub mod risc;
pub mod start;
pub mod test;

use failure::{
    Compat,
    Error as FailureError,
};
use futures::Future;
use telegram_bot::{
    Api,
    types::Message,
};

pub trait Action {
    /// Get the command name for this action.
    ///
    /// The name of the command `/ping` would be `ping`.
    /// The returned value should be in lowercase and must not contain any whitespace.
    fn cmd(&self) -> &'static str;

    /// Check whether this action is for the command with the given name.
    ///
    /// The name of the command `/ping` would be `ping`.
    fn is_cmd(&self, cmd: &str) -> bool {
        cmd.trim().eq_ignore_ascii_case(self.cmd())
    }

    /// Whether this command is hidden from `/help` output and such.
    fn hidden(&self) -> bool;

    /// Short help information for this action.
    fn help(&self) -> &'static str;

    /// Invoke the action with the given context.
    fn invoke(&self, msg: &Message, api: &Api)
        -> Box<Future<Item = (), Error = FailureError>>;
}

/// An action error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while invoking an action.
    #[fail(display = "")]
    //#[fail(display = "failed to invoke action: {}", name)]
    Invoke {
        /// The internal cause of the action error.
        #[cause]
        cause: Compat<FailureError>,

        /// The name of the action.
        name: String,
    }
}
