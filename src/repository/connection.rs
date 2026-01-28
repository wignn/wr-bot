use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;

pub type DbPool = Arc<PgPool>;

pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("[OK] Database connected and migrations applied");
    Ok(Arc::new(pool))
}
