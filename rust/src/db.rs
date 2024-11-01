use jiff::Timestamp;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::{ConnectOptions, FromRow, Row, Sqlite, SqliteConnection};
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
    first_seen: Timestamp,
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
    value: SensorValue,
) -> anyhow::Result<i64> {
    let id = sqlx::query(
        r#"
        INSERT INTO sensor_data (sensor_id, co2, temperature, humidity, lumen, reading_time)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(sensor_id)
    .bind(value.co2)
    .bind(value.temperature)
    .bind(value.humidity)
    .bind(value.lumen)
    .bind(value.reading_time.to_string())
    .execute(conn)
    .await?
    .last_insert_rowid();
    Ok(id)
}

pub async fn get_sensors_with_last_value(
    conn: &mut SqliteConnection,
) -> anyhow::Result<Vec<SensorWithValue>> {
    let sensors = sqlx::query_as(r#"
        SELECT s.id, s.name, s.mac_address, v.co2, v.temperature, v.humidity, v.lumen, v.reading_time
        FROM sensor s
        JOIN sensor_data v on (s.id = v.sensor_id)
        WHERE v.reading_time = (SELECT max(d.reading_time) FROM sensor_data d WHERE d.sensor_id = s.id)
        ORDER by s.name
    "#)
    .fetch_all(conn)
    .await?;
    Ok(sensors)
}

#[derive(Debug, serde::Serialize)]
pub struct SensorValue {
    pub co2: f32,
    pub temperature: f32,
    pub humidity: f32,
    pub lumen: f32,
    pub reading_time: Timestamp,
}

impl FromRow<'_, SqliteRow> for SensorValue {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let reading_time: String = row.try_get("reading_time")?;
        Ok(Self {
            co2: row.try_get("co2")?,
            temperature: row.try_get("temperature")?,
            humidity: row.try_get("humidity")?,
            lumen: row.try_get("lumen")?,
            reading_time: reading_time.parse().unwrap(),
        })
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct SensorWithValue {
    id: i64,
    name: String,
    mac_address: String,
    #[sqlx(flatten)]
    value: SensorValue,
}
