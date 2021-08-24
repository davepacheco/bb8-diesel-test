//!
//! Test whether, when using bb8_diesel, database queries starve execution of
//! the rest of the current task.  We'd expect so, based on the docs.  This
//! example shows the behavior using `tokio::select!`.
//!

use bb8_diesel_test::sleep_using_tokio;
use bb8_diesel_test::sleep_using_db;

#[tokio::main]
async fn main() {
    let manager: bb8_diesel::DieselConnectionManager<diesel::pg::PgConnection> =
        bb8_diesel::DieselConnectionManager::new(
            "postgresql://root@127.0.0.1:32221?sslmode=disable",
        );
    let pool = bb8::Pool::builder().build(manager).await.unwrap();

    eprintln!(
        "TEST ONE: Issue two `tokio::time::sleep` calls using `tokio::select!`."
    );
    eprintln!(
        "Expected behavior: Only the shorter sleep completes.  It takes \n\
        the expected amount of time.  (The other sleep is cancelled.)"
    );
    tokio::select! {
        _ = sleep_using_tokio(1, std::time::Duration::from_millis(500)) => {}
        _ = sleep_using_tokio(2, std::time::Duration::from_millis(300)) => {}
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
        _ = sleep_using_db(3, &pool, std::time::Duration::from_millis(500)) => {}
        _ = sleep_using_tokio(4, std::time::Duration::from_millis(300)) => {}
    };
}
