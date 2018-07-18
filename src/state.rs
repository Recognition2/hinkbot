use std::env;
use std::rc::Rc;

use diesel::{
    mysql::MysqlConnection,
    prelude::*,
};
use telegram_bot::Api;
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
            telegram_client: Self::create_telegram_client(reactor),
            inner: Rc::new(StateInner::init()),
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

    /// Get the stats manager.
    pub fn stats(&self) -> &Stats {
        &self.inner.stats
    }
}

/// The inner state.
struct StateInner {
    /// The database connection.
    db: MysqlConnection,

    /// The stats manager.
    stats: Stats,
}

impl StateInner {
    /// Initialize.
    ///
    /// This initializes the inner state.
    /// Internally this connects to the bot database.
    pub fn init() -> StateInner {
        StateInner {
            db: Self::create_database(),
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
