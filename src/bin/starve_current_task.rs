//!
//! Test whether, when using bb8_diesel, database queries starve execution of
//! the rest of the current task.  We'd expect so, based on the docs.  This
//! example shows the behavior using `tokio::select!`.
//!

use bb8_diesel_test::sleep_using_db_pool;
use bb8_diesel_test::sleep_using_tokio;
use std::time::Duration;

static DATABASE_URL: &str = "postgresql://root@127.0.0.1:32221?sslmode=disable";

#[tokio::main]
async fn main() {
    let manager: bb8_diesel::DieselConnectionManager<diesel::pg::PgConnection> =
        bb8_diesel::DieselConnectionManager::new(DATABASE_URL);
    let pool = bb8::Pool::builder()
        .connection_timeout(Duration::from_secs(3))
        .build(manager)
        .await
        .unwrap();

    // bb8 happily completes this step successfully even when it failed to
    // connect to the database.  Let's catch that and report a better error.
    {
        eprintln!("setting up pool for database {:?}", DATABASE_URL);
        let _ = pool.get().await.unwrap_or_else(|_| {
            panic!("failed to connect to database at {:?}", DATABASE_URL)
        });
    }

    eprintln!(
        "TEST ONE: Issue two `tokio::time::sleep` calls using `tokio::select!`."
    );
    eprintln!(
        "Expected behavior: Only the shorter sleep completes.  It takes \n\
        the expected amount of time.  (The other sleep is cancelled.)"
    );
    tokio::select! {
        _ = sleep_using_tokio(1, Duration::from_millis(500)) => {}
        _ = sleep_using_tokio(2, Duration::from_millis(300)) => {}
    };

    eprintln!(
        "\n\
        TEST TWO: Issue a `tokio::time::sleep` call and a database sleep call\n\
        using `tokio::select!`.\n\
        Expected behavior: We always wait the duration of the database sleep,\n\
        even though it's longer than the other sleep.\n\
        (ideal behavior: the shorter sleep completes first)"
    );
    tokio::select! {
        _ = sleep_using_db_pool(3, &pool, Duration::from_millis(500)) => {}
        _ = sleep_using_tokio(4, Duration::from_millis(300)) => {}
    };
}
