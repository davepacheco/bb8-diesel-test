//! Tests whether you can starve the executor by running database queries.

use bb8_diesel_test::sleep_using_db;
use std::convert::TryFrom;
use std::time::Duration;

/// Number of "core" (worker) threads for the tokio executor
static NTHREADS_CORE: usize = 4;
/// Number of "blocking" threads for the tokio executor
static NTHREADS_BLOCKING: usize = 8;
/// Number of database connections to create and use for sleeps
static NDBCONNS: usize = 4;
/// bb8 pool size
static BB8_POOL_SIZE: u32 = 32;
/// URL for connecting to database
static DATABASE_URL: &str = "postgresql://root@127.0.0.1:32221?sslmode=disable";

fn main() {
    eprintln!("setting up pool for database {:?}", DATABASE_URL);
    let manager: bb8_diesel::DieselConnectionManager<diesel::pg::PgConnection> =
        bb8_diesel::DieselConnectionManager::new(DATABASE_URL);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(NTHREADS_CORE)
        .max_blocking_threads(NTHREADS_BLOCKING)
        .build()
        .unwrap()
        .block_on(async {
            /* Set up a pool and establish all the connections up front. */
            let pool = bb8::Pool::builder()
                .max_size(BB8_POOL_SIZE)
                .min_idle(Some(BB8_POOL_SIZE))
                .connection_timeout(Duration::from_secs(3))
                .build(manager)
                .await
                .unwrap_or_else(|_| {
                    panic!("failed to establish connection to database")
                });
            let start = std::time::Instant::now();
            let mut wait = Vec::new();

            /*
             * Spawn off NDBCONNS tokio tasks, each of which will acquire a
             * database connection and execute a query that sleeps for one
             * second.
             *
             * In between these tasks, spawn a task that can complete
             * immediately.
             *
             * If it's possible to starve the entire executor, we'll stop seeing
             * these intermediate tasks complete for a period while database
             * tasks are still running.  When the database tasks finish, we'll
             * resume (and finish) the intermediate tasks.
             *
             * In a fully async implementation, this wouldn't be possible:
             * database tasks could starve each other if we run out of blocking
             * threads, but it should never be possible to delay these quick
             * tasks.
             */
            for i in 0..NDBCONNS {
                let conn = pool.get_owned().await.unwrap_or_else(|_| {
                    panic!("failed to establish connection to database")
                });
                wait.push(tokio::spawn(async move {
                    sleep_using_db(
                        u8::try_from(i).unwrap(),
                        &conn,
                        std::time::Duration::from_millis(1000),
                    )
                }));
                wait.push(tokio::spawn(async move {
                    eprintln!("{:?} intermediate task {}", start.elapsed(), i);
                }));
            }

            /*
             * Kick off a bunch more quick tasks for more visibility into
             * liveness.
             */
            for i in 0..NDBCONNS {
                wait.push(tokio::spawn(async move {
                    eprintln!("{:?} later task {}", start.elapsed(), i);
                }));
            }

            futures::future::join_all(wait).await;
        });
}
