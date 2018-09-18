pub mod isolated;
pub mod normal;

use std::io::Error as IoError;

/// An executor error.
#[derive(Debug, Fail)]
pub enum Error {
    /// An error occurred while spawning the user command.
    #[fail(display = "failed to spawn user command for execution")]
    Spawn(#[cause] IoError),

    /// An error occurred while collecting output from the invoked user command.
    #[fail(display = "failed to collect user command output")]
    CollectOutput(#[cause] IoError),

    // /// An error occurred while processing output from the invoked user command.
    // #[fail(display = "failed to process user command output")]
    // ProcessOutput(#[cause] IoError),
    /// An error occurred while waiting for the spawned user command complete.
    #[fail(display = "failed to wait for user command to complete")]
    Complete(#[cause] IoError),
}
