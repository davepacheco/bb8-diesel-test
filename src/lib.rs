#[macro_use]
extern crate diesel;
use diesel::RunQueryDsl;

// Expose the "pg_sleep" SQL function to Diesel.  This function is implemented
// in PostgreSQL and CockroachDB to sleep for the requested number of seconds.
diesel::sql_function! {
    fn pg_sleep(seconds: diesel::sql_types::Float) -> diesel::sql_types::Bool;
}

///
/// Returns a Future that will sleep for the requested `duration` using
/// [`tokio::time::sleep`].  Prints information to show when it starts and stops
/// sleeping and how long it took.
///
pub async fn sleep_using_tokio(id: u8, duration: std::time::Duration) {
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
pub async fn sleep_using_db_pool(
    id: u8,
    pool: &bb8::Pool<
        bb8_diesel::DieselConnectionManager<diesel::pg::PgConnection>,
    >,
    duration: std::time::Duration,
) {
    let conn = pool.get().await.unwrap();
    sleep_using_db(id, &*conn, duration);
}

///
/// Like [`sleep_using_db_pool`], but accepts the connection directly.
///
pub fn sleep_using_db(
    id: u8,
    conn: &diesel::pg::PgConnection,
    duration: std::time::Duration,
) {
    eprintln!("[{}] begin db sleep for {:?}", id, duration);
    let start = std::time::Instant::now();
    diesel::select(pg_sleep(duration.as_secs_f32()))
        .load::<bool>(conn)
        .unwrap();
    eprintln!(
        "[{}] database sleep for {:?}, slept for {:?}",
        id,
        duration,
        start.elapsed()
    );
}
