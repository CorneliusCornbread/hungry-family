use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, postgres::PgConnectOptions};
use std::str::FromStr;
use url::Url;

/// Connect to the maintenance database (`postgres`) and create the target
/// database from `DATABASE_URL` if it does not exist yet.
///
/// The connecting role needs `CREATEDB` privileges.
pub async fn ensure_database(database_url: &str) -> Result<(), sqlx::Error> {
    let target = Url::parse(database_url)
        .map_err(|e| sqlx::Error::Configuration(format!("invalid DATABASE_URL: {e}").into()))?;

    let db_name = target
        .path()
        .strip_prefix('/')
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            sqlx::Error::Configuration("DATABASE_URL must include a database name".into())
        })?
        .to_owned();

    let mut conn = PgConnection::connect_with(
        &PgConnectOptions::from_str(database_url)?.database("postgres"),
    )
        .await?;

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(&db_name)
            .fetch_one(&mut conn)
            .await?;

    if !exists {
        if !is_valid_db_name(&db_name) {
            return Err(sqlx::Error::Protocol("Invalid database name".into()));
        }

        tracing::info!("Database \"{db_name}\" does not exist, creating it");
        let query = format!("CREATE DATABASE \"{}\"", db_name.replace('"', "\"\""));

        sqlx::query(AssertSqlSafe(query))
            .execute(&mut conn)
            .await?;
    }

    Ok(())
}

fn is_valid_db_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !name.starts_with(|c: char| c.is_numeric())
}

/// Apply all pending embedded migrations from `migrations/`.
///
/// Applied migrations are tracked in a `_sqlx_migrations` table, so calling
/// this on every startup is a no-op once up to date.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!().run(pool).await
}
