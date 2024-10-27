#![forbid(unsafe_code)]

use axum::extract::State;
use axum::response::Html;
use axum::{extract::Query, routing::get, Router};
use db::{Database, SensorValue};
use jiff::{RoundMode, Timestamp, TimestampRound, Unit};
use serde::Deserialize;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database = setup_database().await?;
    database.run_migrations().await?;

    let app = Router::new()
        .route("/co2-ampel", get(receive_sensor_values))
        .route("/", get(index))
        .with_state(database);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await?;
    Ok(())
}

async fn setup_database() -> anyhow::Result<Database> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = Database::new(&database_url).await?;
    Ok(pool)
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Params {
    id: String,
    #[serde(rename = "c")]
    co2: f32,
    #[serde(rename = "t")]
    temperature: f32,
    #[serde(rename = "h")]
    humidity: f32,
    #[serde(rename = "l")]
    lumen: f32,
}

async fn receive_sensor_values(
    State(database): State<Database>,
    Query(params): Query<Params>,
) -> &'static str {
    tracing::debug!("data received: {params:?}");
    let now = current_time_millis();
    let mut conn = database.get_connection().await.unwrap();
    let sensor_id: Option<i64> = db::find_sensor_id(&mut *conn, &params.id).await.unwrap();
    let sensor_id = if let Some(id) = sensor_id {
        id
    } else {
        db::insert_sensor(&mut *conn, &params.id, now)
            .await
            .unwrap()
    };
    let value = SensorValue {
        co2: params.co2,
        temperature: params.temperature,
        humidity: params.humidity,
        lumen: params.lumen,
        reading_time: now,
    };
    db::insert_sensor_value(&mut *conn, sensor_id, value)
        .await
        .unwrap();
    tracing::debug!("sensor ID: {sensor_id}");
    "done"
}

async fn index(State(database): State<Database>) -> Html<String> {
    let mut conn = database.get_connection().await.unwrap();
    let sensors = db::get_sensors_with_last_value(&mut *conn).await.unwrap();
    Html(format!("<html><h1>Hello</h1><p>{sensors:?}</html>"))
}

fn current_time_millis() -> Timestamp {
    Timestamp::now()
        .round(
            TimestampRound::new()
                .smallest(Unit::Millisecond)
                .mode(RoundMode::Floor),
        )
        .expect("Rounding to milliseconds should never fail")
}
