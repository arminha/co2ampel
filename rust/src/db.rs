use jiff::Timestamp;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, Sqlite, SqliteConnection};
use std::str::FromStr;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let db_options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .disable_statement_logging()
            .to_owned();
        let pool = SqlitePoolOptions::new().connect_with(db_options).await?;
        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        tracing::info!("run migrations");
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub async fn get_connection(&self) -> anyhow::Result<PoolConnection<Sqlite>> {
        let conn = self.pool.acquire().await?;
        Ok(conn)
    }
}

pub async fn find_sensor_id(
    conn: &mut SqliteConnection,
    mac_address: &str,
) -> anyhow::Result<Option<i64>> {
    let id = sqlx::query_scalar(
        r#"
        SELECT id
        FROM sensor
        WHERE mac_address = $1
        "#,
    )
    .bind(mac_address)
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

pub async fn insert_sensor(
    conn: &mut SqliteConnection,
    mac_address: &str,
    first_seen: &Timestamp,
) -> anyhow::Result<i64> {
    let id = sqlx::query(
        r#"
        INSERT INTO sensor (mac_address, name, first_seen)
        VALUES ($1, $1, $2)
        "#,
    )
    .bind(mac_address)
    .bind(first_seen.to_string())
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}

pub async fn insert_sensor_value(
    conn: &mut SqliteConnection,
    sensor_id: i64,
    co2: f32,
    temperature: f32,
    humidity: f32,
    lumen: f32,
    reading_time: &Timestamp,
) -> anyhow::Result<i64> {
    let id = sqlx::query(
        r#"
        INSERT INTO sensor_data (sensor_id, co2, temperature, humidity, lumen, reading_time)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(sensor_id)
    .bind(co2)
    .bind(temperature)
    .bind(humidity)
    .bind(lumen)
    .bind(reading_time.to_string())
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}
