///
/// Test whether, when using bb8_diesel, database queries starve execution of
/// the rest of the current task.  We'd expect so, based on the docs.  This
/// example shows the behavior using `tokio::select!`.
///

#[macro_use]
extern crate diesel;
use diesel::RunQueryDsl;

// Expose the "pg_sleep" SQL function to Diesel.  This function is implemented
// in PostgreSQL and CockroachDB to sleep for the requested number of seconds.
diesel::sql_function! {
    fn pg_sleep(seconds: diesel::sql_types::Float) -> diesel::sql_types::Bool;
}

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

///
/// Returns a Future that will sleep for the requested `duration` using
/// [`tokio::time::sleep`].  Prints information to show when it starts and stops
/// sleeping and how long it took.
///
async fn sleep_using_tokio(id: u8, duration: std::time::Duration) {
    let start = std::time::Instant::now();

    eprintln!("[{}] begin tokio sleep for {:?}", id, duration);
    tokio::time::sleep(duration).await;
    eprintln!(
        "[{}] done tokio sleep for {:?}, slept for {:?}",
        id,
        duration,
        start.elapsed()
    );
}

///
/// Like [`sleep_using_tokio`], but makes a database query to sleep instead of
/// using [`tokio::time::sleep`].
///
async fn sleep_using_db(
    id: u8,
    pool: &bb8::Pool<
        bb8_diesel::DieselConnectionManager<diesel::pg::PgConnection>,
    >,
    duration: std::time::Duration,
) {
    let c = pool.get().await.unwrap();
    eprintln!("[{}] begin db sleep for {:?}", id, duration);
    let start = std::time::Instant::now();
    diesel::select(pg_sleep(duration.as_secs_f32())).load::<bool>(&*c).unwrap();
    eprintln!(
        "[{}] database sleep for {:?}, slept for {:?}",
        id,
        duration,
        start.elapsed()
    );
}
