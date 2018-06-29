extern crate tokio_io;
extern crate tokio_process;

use std::io::BufReader;
use std::process::{Command, ExitStatus, Stdio};

use futures::{
    Future,
    Stream,
};
use self::tokio_io::io::lines;
use self::tokio_process::CommandExt;

/// Execute the given command.
/// 
/// `stdout` and `stderr` is streamed line by line to the `output` closure,
/// which is called for each line that received.
pub fn execute<O>(cmd: &mut Command, output: O)
    -> Box<Future<Item = ExitStatus, Error = ()>>
    where
        O: Fn(String) -> Result<(), ()> + Clone + 'static
{
    // Spawn an isolated container to run the user command in
    // TODO: configurable timeout
    // TODO: also handle a timeout fallback outside the actual container
    // TODO: map container UIDs to something above 10k
    let mut process = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn_async()
        .unwrap();

    // Build an stdout and stderr reader to process output
    let stdout_reader = BufReader::new(process.stdout().take().unwrap());
    let stderr_reader = BufReader::new(process.stderr().take().unwrap());
    let stdout_stream = lines(stdout_reader).map_err(|_| ()).for_each(output.clone());
    let stderr_stream = lines(stderr_reader).map_err(|_| ()).for_each(output);

    // Create a future for when the process exists and the status code is known
    let process_exit = process.wait_with_output()
        .map(|output| output.status)
        .map_err(|_| ());

    // Create a future for when running the process fully completes
    let process_complete = process_exit
        .join3(stdout_stream, stderr_stream)
        .map(|(status, _, _)| status);

    Box::new(process_complete)
}
