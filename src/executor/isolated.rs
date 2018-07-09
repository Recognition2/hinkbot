use std::process::{Command, ExitStatus};

use futures::Future;

use super::{
    Error,
    normal,
};

/// Execute the given command in a secure isolated environment.
///
/// `stdout` and `stderr` is streamed line by line to the `output` closure,
/// which is called for each line that received.
pub fn execute<O>(cmd: String, output: O)
    -> impl Future<Item = ExitStatus, Error = Error>
    where
        O: Fn(String) -> Result<(), Error> + Clone + 'static
{
    // Use Docker as base command
    let mut isolated_cmd = Command::new("docker");

    // Configure Docker and set a timeout for the command to run
    // TODO: configurable timeout
    // TODO: also handle a timeout fallback outside the actual container
    // TODO: map container UIDs to something above 10k
    let isolated_cmd = isolated_cmd.arg("run")
        .arg("--rm")
        .args(&["--cpus", "0.2"])
        // TODO: enable these memory limits once the warning is fixed
        // .args(&["--memory", "100m"])
        // .args(&["--kernel-memory", "25m"])
        // .args(&["--memory-swappiness", "0"])
        // .args(&["--device-read-bps", "/:50mb"])
        // .args(&["--device-write-bps", "/:50mb"])
        .args(&["--workdir", "/root"])
        .args(&["--restart", "no"])
        .args(&["--stop-timeout", "1"])
        .arg("risc-exec")
        .args(&["timeout", "--signal=SIGTERM", "--kill-after=305", "300"])
        .args(&["bash", "-c", &cmd]);

    // Execute the isolated command in the normal environment
    normal::execute(isolated_cmd, output)
}
