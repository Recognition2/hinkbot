extern crate htmlescape;
extern crate tokio_io;
extern crate tokio_process;
extern crate tokio_timer;

use std::io::BufReader;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use futures::{
    future::{err, ok},
    Future,
    Stream,
};
use telegram_bot::{
    Api,
    prelude::*,
    types::{Message, MessageKind, ParseMode},
};
use self::htmlescape::encode_minimal;
use self::tokio_io::io::lines;
use self::tokio_process::CommandExt;
use self::tokio_timer::Interval;

use super::Action;

const CMD: &'static str = "exec";

pub struct Exec;

impl Exec {
    pub fn new() -> Self {
        Exec
    }
}

impl Action for Exec {
    fn cmd(&self) -> &'static str {
        CMD
    }

    // TODO: proper error handling everywhere, pass errors along
    fn invoke(&self, msg: &Message, api: &Api) -> Box<Future<Item = (), Error = ()>> {
        if let MessageKind::Text {
            ref data,
            ..
        } = &msg.kind {

            // The Telegram API client to use
            let api = api.clone();

            // The command to run in the shell
            // TODO: actually properly fetch the command to execute from the full message
            let command = data.splitn(2, ' ')
                .skip(1)
                .next()
                .map(|cmd| cmd.trim_left())
                .unwrap_or("")
                .to_owned();

            // Provide the user with feedback if no command is entered
            if command.trim().is_empty() {
                api.spawn(
                    msg.text_reply("\
                        Please provide a command to run.\n\
                        \n\
                        For example:\n\
                        `/exec echo Hello!`\
                    ").parse_mode(ParseMode::Markdown),
                );
                return Box::new(ok(()));
            }

            // Print the command to run
            println!("CMD: {}", command);

            // Create the status message, and build the executable status object
            let exec_status = ExecStatus::create_status_msg(msg, api.clone());
            let exec_status = exec_status.and_then(move |exec_status| {
                // Create an mutexed arc for the exec status
                let exec_status = Arc::new(Mutex::new(exec_status));

                // Spawn an isolated container to run the user command in
                // TODO: configurable timeout
                // TODO: also handle a timeout fallback outside the actual container
                // TODO: map container UIDs to something above 10k
                let mut process = Command::new("docker")
                    .arg("run")
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
                    .arg("ubuntu")
                    .args(&["timeout", "--signal=SIGTERM", "--kill-after=65", "60"])
                    .args(&["bash", "-c", &command])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn_async()
                    .unwrap();

                // Build an stdout and stderr reader to process output
                let stdout_reader = BufReader::new(process.stdout().take().unwrap());
                let stderr_reader = BufReader::new(process.stderr().take().unwrap());
                let exec_status_stdout = exec_status.clone();
                let exec_status_stderr = exec_status.clone();
                let stdout_stream = lines(stdout_reader)
                    .for_each(move |line| {
                        // Append the line to the output
                        exec_status_stdout.lock().unwrap().append_line(&line);
                        Ok(())
                    })
                    .map_err(|_| ());
                let stderr_stream = lines(stderr_reader)
                    .for_each(move |line| {
                        // Append the line to the output
                        exec_status_stderr.lock().unwrap().append_line(&line);
                        Ok(())
                    })
                    .map_err(|_| ());

                // Create a future for when the process exists and the status code is known
                let exec_status_complete = exec_status.clone();
                let process_exit = process.wait_with_output()
                    .and_then(move |output| {
                        // Update the status
                        exec_status_complete.lock().unwrap().set_status(output.status);
                        ok(())
                    })
                    .map_err(|_| ());

                // Create a future for when running the process fully completes
                let process_complete = stdout_stream
                    .join(stderr_stream)
                    .join(process_exit)
                    .map(|_| ());

                // Set up an interval for constantly updating the status
                let exec_status_interval = exec_status.clone();
                Interval::new(
                        Instant::now() + Duration::from_millis(500),
                        Duration::from_millis(500),
                    )
                    .for_each(move |_| {
                        exec_status_interval.lock().unwrap().update_throttled();
                        Ok(())
                    })
                    .map_err(|_| ())
                    .select(process_complete)
                    .and_then(move |_| {
                        exec_status.lock().unwrap().update();
                        ok(())
                    })
                    .map_err(|_| ())
            });

            return Box::new(exec_status);
        }

        Box::new(ok(()))
    }
}

/// An object that tracks the status of an executed command.
/// This object also holds the status message present in a Telegram group to update when the status
/// changes, along with an Telegram API instance.
// TODO: detect timeouts
// TODO: report execution times
struct ExecStatus {
    /// The actual output.
    output: String,

    /// The exit status of the process.
    /// If set, the execution has completed although it might have failed.
    /// The status code itself defines whether the execution was successful.
    status: Option<ExitStatus>,

    /// True if the output or status has changed since the last status message update.
    /// If true, this means that the status message doesn't represent the current status corretly,
    /// and thus it should be updated.
    changed: bool,

    /// The time the Telegram status message was last changed at.
    /// When the status instance is created, this is set to the current time.
    /// This is used to manage throttling.
    changed_at: SystemTime,

    /// An Telegram API client to update the status message with.
    api: Api,

    /// The status message in a Telegram chat that should be updated to report the executing
    /// status.
    status_msg: Message,
}

impl ExecStatus {
    /// Create a status output message as reply on the given `msg`,
    /// and return an `ExecStatus` for it.
    pub fn create_status_msg(msg: &Message, api: Api)
        -> impl Future<Item = Self, Error = ()>
    {
        // TODO: make this timeout configurable
        // TODO: handle the Telegram errors properly
        api.send_timeout(
                msg.text_reply("<i>Executing command...</i>")
                    .parse_mode(ParseMode::Html),
                Duration::from_secs(10),
            )
            .map_err(|err| println!("TELEGRAM ERROR: {:?}", err))
            .and_then(|msg| if let Some(msg) = msg {
                ok(ExecStatus::new(msg, api))
            } else {
                err(())
            })
            .map_err(|_| println!("TELEGRAM ERROR: no message"))
    }

    /// Build a new exec status object with the given status message and Telegram API client
    /// instance.
    pub fn new(status_msg: Message, api: Api) -> Self {
        ExecStatus {
            output: String::new(),
            status: None,
            changed: false,
            changed_at: SystemTime::now(),
            api,
            status_msg,
        }
    }

    /// Append the given `output` to the cummulative output.
    /// Note that if the output is getting too large, parts will be truncated.
    /// To append a line, use `append_line()` instead.
    pub fn append(&mut self, output: &str) {
        let truncate_at = 4096 - 100;

        // Do not append if the output became too large
        if self.output.len() > truncate_at {
            return;
        }

        // Append the output
        self.output += output;

        // Truncate the output if it became too big
        if self.output.len() >= truncate_at {
            self.output.truncate(truncate_at);
            self.output += " [truncated]";
        }

        // If anything is appended, we've changed
        if !output.is_empty() {
            self.changed = true;
        }
    }

    /// Append the given `line` to the output.
    /// The given line should not include a newline character.
    /// Note that if the output is getting too large, parts will be truncated.
    pub fn append_line(&mut self, line: &str) {
        if !self.output.is_empty() {
            self.append("\n");
        }
        self.append(line);
    }

    /// Set the exit status of the executed command.
    pub fn set_status(&mut self, status: ExitStatus) {
        // Mark that the status has changed if the exit status is different
        if self.status != Some(status) {
            self.changed = true;
        }

        // Update the status
        self.status = Some(status);
    }

    /// Check whether this executable has completed.
    /// It may have successfully completed or it may have failed.
    pub fn completed(&self) -> bool {
        self.status.is_some()
    }

    /// Build the status message contents, based on the current executing status.
    /// The returned status message is in HTML format.
    fn build_status_msg(&self) -> String {
        // If not completed, and there is no output yet
        if !self.completed() && self.output.is_empty() {
            return "<i>Executing command...</i>".into();
        }

        // Determine what status emoji to use
        let emoji = if !self.completed() {
            "⏳"
        } else if self.status.unwrap().success() {
            "✅"
        } else {
            "❌"
        };

        // Deterime whether to print a special notice
        let notice = match self.status {
            Some(status) if !status.success() =>
                format!(
                    "   Exit code <code>{}</code>",
                    status.code()
                        .map(|code| code.to_string())
                        .unwrap_or("?".into()),
                ),
            _ => String::new(),
        };

        // Format the output
        let output = if self.output.is_empty() {
            "<i>No output</i>".to_owned()
        } else {
            format!("\
                    <b>Output:</b>\n\
                    <code>{}</code>\
                ",
                encode_minimal(&self.output),
            )
        };

        // Format the message
        format!("\
                {}\n\
                \n\
                {}{}\
            ",
            output,
            emoji,
            notice,
        )
    }

    /// Update the status message in Telegram with the newest status data in this object.
    /// This method spawns a future that completes asynchronously.
    pub fn update_status_msg(&mut self) {
        // Spawn a future to edit the status message with the newest build status text
        self.api.spawn(
            self.status_msg
                .edit_text(self.build_status_msg())
                .parse_mode(ParseMode::Html)
        );

        // Reset the changed status
        self.changed = false;
        self.changed_at = SystemTime::now();
    }

    /// Update the status message in Telegram with the newest status data in this object,
    /// if it has been changed after the last update.
    pub fn update(&mut self) {
        // Only if something changed
        if !self.changed {
            return;
        }

        // Actually update
        self.update_status_msg()
    }

    /// Update the status message in Telegram with the newest status data in this object,
    /// if it has been changed after the last update.
    ///
    /// This method won't update if it was invoked too quickly before the last change.
    pub fn update_throttled(&mut self) {
        // Throttle
        // TODO: make the throttle time configurable
        match self.changed_at.elapsed() {
            Ok(elapsed) if elapsed < Duration::from_millis(495) => return,
            Err(..) => return,
            _ => {},
        }

        // Update
        self.update()
    }
}
