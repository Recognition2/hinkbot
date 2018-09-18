extern crate tokio_io;
extern crate tokio_process;

use std::io::BufReader;
use std::process::{Command, ExitStatus, Stdio};

use self::tokio_io::io::lines;
use self::tokio_process::CommandExt;
use super::Error;
use futures::{future::err, Future, Stream};

/// Execute the given command.
///
/// `stdout` and `stderr` is streamed line by line to the `output` closure,
/// which is called for each line that received.
pub fn execute<O>(cmd: &mut Command, output: O) -> Box<Future<Item = ExitStatus, Error = Error>>
where
    O: Fn(String) -> Result<(), Error> + Clone + 'static,
{
    // Spawn a child process to run the given command in
    // TODO: configurable timeout
    let process = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn_async();

    // Return errors
    let mut process = match process {
        Ok(process) => process,
        Err(e) => return Box::new(err(Error::Spawn(e))),
    };

    // Build process output streams, process each line with the output closure
    let stdout_reader = BufReader::new(process.stdout().take().unwrap());
    let stderr_reader = BufReader::new(process.stderr().take().unwrap());
    let stdout_stream = lines(stdout_reader)
        .map_err(|err| Error::CollectOutput(err))
        .for_each(output.clone());
    let stderr_stream = lines(stderr_reader)
        .map_err(|err| Error::CollectOutput(err))
        .for_each(output);

    // Wait for the child process to exit, catch the status code
    let process_exit = process
        .wait_with_output()
        .map(|output| output.status)
        .map_err(|err| Error::Complete(err));

    // Wait on the output streams and on a status code, return the future
    Box::new(
        process_exit
            .join3(stdout_stream, stderr_stream)
            .map(|(status, _, _)| status),
    )
}
