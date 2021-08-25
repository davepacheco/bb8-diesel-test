//!
//! Test how bb8 behaves when a database is offline.
//!

use anyhow::Context;
use diesel::expression::AsExpression;
use diesel::pg::PgConnection;
use diesel::sql_types::Integer;
use diesel::RunQueryDsl;
use std::time::Duration;

static DATABASE_URL_OK: &str =
    "postgresql://root@127.0.0.1:32221?sslmode=disable";
static DATABASE_URL_BAD: &str =
    "postgresql://root@127.0.0.1:32222?sslmode=disable";

#[tokio::main]
async fn main() {
    eprintln!("try connecting to working database");
    try_connect(DATABASE_URL_OK, 1).await.unwrap();
    eprintln!("\n\ntry connecting to non-working database (min_idle = 0)");
    let error = try_connect(DATABASE_URL_BAD, 0).await.unwrap_err();
    eprintln!("{:?}", error);
    eprintln!("\n\ntry connecting to non-working database (min_idle = 1)");
    let error = try_connect(DATABASE_URL_BAD, 1).await.unwrap_err();
    eprintln!("{:?}", error);
}

async fn try_connect(url: &str, min_idle: u32) -> Result<(), anyhow::Error> {
    let pool = make_pool(url, min_idle).await?;
    eprintln!("pool state: {:?}", pool.state());
    eprintln!("getting connection ... ");
    let conn = pool.get().await.context("acquiring connection")?;
    eprintln!("got one!");
    eprintln!("running query ... ");
    let ten = diesel::select(AsExpression::<Integer>::as_expression(10_i32))
        .load::<i32>(&*conn)
        .context("running query")?;
    eprintln!("query okay: {:?}", ten);
    Ok(())
}

async fn make_pool(
    url: &str,
    min_idle: u32,
) -> Result<
    bb8::Pool<bb8_diesel::DieselConnectionManager<PgConnection>>,
    anyhow::Error,
> {
    eprintln!("creating pool (min_idle={}) for url: {:?}", min_idle, url);
    let manager: bb8_diesel::DieselConnectionManager<PgConnection> =
        bb8_diesel::DieselConnectionManager::new(url);
    let mut builder =
        bb8::Pool::builder().connection_timeout(Duration::from_secs(3));

    if min_idle > 0 {
        builder = builder.min_idle(Some(min_idle));
    }

    Ok(builder.build(manager).await.context("building pool")?)
}
