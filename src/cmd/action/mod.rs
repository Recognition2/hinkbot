pub mod exec;
pub mod help;
pub mod id;
pub mod ping;
pub mod risc;
pub mod start;
pub mod stats;
pub mod test;

use failure::{
    Compat,
    Error as FailureError,
};
use futures::Future;
use telegram_bot::types::Message;

use state::State;

lazy_static! {
    /// A list of all available and invokable actions.
    /// This list includes hidden actions which may be filtered using the `.hidden()` propery.
    pub(crate) static ref ACTIONS: Vec<Box<dyn Action + Sync>> = vec![
        Box::new(self::exec::Exec::new()),
        Box::new(self::help::Help::new()),
        Box::new(self::id::Id::new()),
        Box::new(self::ping::Ping::new()),
        Box::new(self::risc::Risc::new()),
        Box::new(self::start::Start::new()),
        Box::new(self::stats::Stats::new()),
        Box::new(self::test::Test::new()),
    ];
}

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
    fn invoke(&self, state: &State, sg: &Message)
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
