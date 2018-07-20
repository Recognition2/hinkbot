use std::env;
use std::rc::Rc;
use std::time::Duration;

use diesel::{
    mysql::MysqlConnection,
    prelude::*,
};
use futures::Future;
use telegram_bot::{
    Api,
    Error as TelegramError,
    types::{JsonIdResponse, Message, Request},
};
use tokio_core::reactor::Handle;

use stats::Stats;

/// The global application state.
#[derive(Clone)]
pub struct State {
    /// The Telegram API client beign used.
    telegram_client: Api,

    /// The inner state.
    inner: Rc<StateInner>,
}

impl State {
    /// Initialize.
    ///
    /// This initializes the global state.
    /// Internally this creates the Telegram API client and sets up a connection,
    /// connects to the bot database and more.
    ///
    /// A handle to the Tokio core reactor must be given to `reactor`.
    pub fn init(reactor: Handle) -> State {
        State {
            telegram_client: Self::create_telegram_client(reactor.clone()),
            inner: Rc::new(StateInner::init(reactor)),
        }
    }

    /// Create a Telegram API client instance, and initiate a connection.
    fn create_telegram_client(reactor: Handle) -> Api {
        // Retrieve the Telegram bot token
        let token = env::var("TELEGRAM_BOT_TOKEN")
            .expect("env var TELEGRAM_BOT_TOKEN not set");

        // Initiate the Telegram API client, and return
        Api::configure(token)
            .build(reactor)
            .expect("failed to initialize Telegram API client")
    }

    /// Get the database connection.
    pub fn db(&self) -> &MysqlConnection {
        &self.inner.db
    }

    /// Get the Telegram API client.
    pub fn telegram_client(&self) -> &Api {
        &self.telegram_client
    }

    /// Send a request using the Telegram API client, and track the messages the bot sends.
    /// Because the stats of this message need to be tracked, it only allows to send requests that
    /// have a `Message` as response.
    /// This function uses a fixed timeout internally.
    pub fn telegram_send<Req>(&self, request: Req)
        -> Box<Future<Item = Option<Message>, Error = TelegramError>>
        where
            Req: Request<Response = JsonIdResponse<Message>>,
    {
        // Clone the state for use in this future
        let state = self.clone();

        // Send the message through the Telegram client, track the response for stats
        let future = self.telegram_client()
            .send_timeout(request, Duration::from_secs(10))
            .inspect(move |msg| if let Some(msg) = msg {
                if msg.edit_date.is_none() {
                    state.stats().increase_stats(msg, 1, 0);
                } else {
                    state.stats().increase_stats(msg, 0, 1);
                }
            });

        Box::new(future)
    }

    /// Send a request using the Telegram API client, and track the messages the bot sends.
    /// This function spawns the request on the background and runs it to completion.
    /// Because the stats of this message need to be tracked, it only allows to send requests that
    /// have a `Message` as response.
    /// This function uses a fixed timeout internally.
    pub fn telegram_spawn<Req>(&self, request: Req)
        where
            Req: Request<Response = JsonIdResponse<Message>>,
    {
        self.inner.handle.spawn(
            self.telegram_send(request).then(|_| Ok(())),
        )
    }

    /// Get the stats manager.
    pub fn stats(&self) -> &Stats {
        &self.inner.stats
    }
}

/// The inner state.
struct StateInner {
    /// The database connection.
    db: MysqlConnection,

    /// A handle to the reactor.
    handle: Handle,

    /// The stats manager.
    stats: Stats,
}

impl StateInner {
    /// Initialize.
    ///
    /// This initializes the inner state.
    /// Internally this connects to the bot database.
    pub fn init(handle: Handle) -> StateInner {
        StateInner {
            db: Self::create_database(),
            handle,
            stats: Stats::new(),
        }
    }

    /// Create a MySQL connection to the database.
    fn create_database() -> MysqlConnection {
        // Retrieve the database connection URL
        let database_url = env::var("DATABASE_URL")
            .expect("env var DATABASE_URL not set");

        // Connect to the database
        MysqlConnection::establish(&database_url)
            .expect(&format!("Failed to connect to database on {}", database_url))
    }
}
