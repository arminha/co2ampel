#![forbid(unsafe_code)]

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use db::{Database, SensorValue};
use headers::HeaderMapExt;
use jiff::{civil::Date, tz::TimeZone, RoundMode, Timestamp, TimestampRound, ToSpan, Unit};
use minijinja::{context, Environment};
use serde::Deserialize;
use static_content::StaticContent;
use std::{
    env,
    sync::{Arc, LazyLock},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod static_content;

const INDEX_TT: &str = include_str!("assets/index.html");
const SENSOR_TT: &str = include_str!("assets/sensor.html");
const TEXT_CSS: &str = "text/css";
static STYLE_CSS: LazyLock<StaticContent> =
    LazyLock::new(|| StaticContent::new(include_str!("assets/css/style.css"), TEXT_CSS));
static BOOTSTRAP_CSS: LazyLock<StaticContent> =
    LazyLock::new(|| StaticContent::new(include_str!("assets/css/bootstrap-4.3.1.css"), TEXT_CSS));

#[derive(Clone)]
struct AppState {
    database: Database,
    env: Arc<Environment<'static>>,
}

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

    let mut env = Environment::new();
    env.add_template("index.html", INDEX_TT)?;
    env.add_template("sensor.html", SENSOR_TT)?;
    env.add_filter("datetime", format_timestamp);
    let env = Arc::new(env);

    let app = Router::new()
        .route("/co2-ampel", get(receive_sensor_values))
        .route("/", get(index))
        .route("/sensor/{id}", get(sensor_detail))
        .route("/css/style.css", get(style_css))
        .route("/css/bootstrap-4.3.1.css", get(bootstrap_css))
        .with_state(AppState { database, env });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::debug!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

fn format_timestamp(value: String) -> String {
    let timestamp: Timestamp = value.parse().unwrap();
    let zoned = timestamp
        .to_zoned(TimeZone::system())
        .round(Unit::Second)
        .unwrap();
    zoned.strftime("%H:%M:%S %d.%m.%Y").to_string()
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
    State(app_state): State<AppState>,
    Query(params): Query<Params>,
) -> &'static str {
    tracing::debug!("data received: {params:?}");
    let now = current_time_millis();
    let mut conn = app_state.database.get_connection().await.unwrap();
    let sensor_id: Option<i64> = db::find_sensor_id(&mut conn, &params.id).await.unwrap();
    let sensor_id = if let Some(id) = sensor_id {
        id
    } else {
        db::insert_sensor(&mut conn, &params.id, now).await.unwrap()
    };
    let value = SensorValue {
        co2: params.co2,
        temperature: params.temperature,
        humidity: params.humidity,
        lumen: params.lumen,
        reading_time: now,
    };
    db::insert_sensor_value(&mut conn, sensor_id, value)
        .await
        .unwrap();
    tracing::debug!("sensor ID: {sensor_id}");
    "done"
}

async fn index(State(app_state): State<AppState>) -> Html<String> {
    let mut conn = app_state.database.get_connection().await.unwrap();
    let sensors = db::get_sensors_with_last_value(&mut conn).await.unwrap();
    let template = app_state.env.get_template("index.html").unwrap();
    let html = template
        .render(context! {
            sensors => sensors,
        })
        .unwrap();
    Html(html)
}

#[derive(Deserialize, Debug)]
struct DetailParams {
    date: Option<Date>,
}

async fn sensor_detail(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<DetailParams>,
) -> Result<Html<String>, StatusCode> {
    let mut conn = app_state.database.get_connection().await.unwrap();
    let sensor = db::get_sensor_with_last_value(&mut conn, id).await.unwrap();
    let sensor = match sensor {
        None => return Err(StatusCode::NOT_FOUND),
        Some(sensor) => sensor,
    };
    let (from, to) = if let Some(date) = params.date {
        let zoned = date.to_zoned(TimeZone::system()).unwrap();
        (
            zoned.start_of_day().unwrap().timestamp(),
            zoned.end_of_day().unwrap().timestamp(),
        )
    } else {
        (
            sensor.value.reading_time - 24.hours(),
            sensor.value.reading_time,
        )
    };
    let values = db::get_sensor_values(&mut conn, id, from, to)
        .await
        .unwrap();

    let template = app_state.env.get_template("sensor.html").unwrap();
    let (prev, next) = if let Some(date) = params.date {
        (date.yesterday().unwrap(), Some(date.tomorrow().unwrap()))
    } else {
        (
            sensor
                .value
                .reading_time
                .to_zoned(TimeZone::system())
                .date()
                .yesterday()
                .unwrap(),
            None,
        )
    };
    let html = template
        .render(context! {
            sensor => sensor,
            values => values,
            prev => prev,
            next => next,
        })
        .unwrap();
    Ok(Html(html))
}

async fn style_css(headers: HeaderMap) -> impl IntoResponse {
    STYLE_CSS.get_request(headers.typed_get())
}

async fn bootstrap_css(headers: HeaderMap) -> impl IntoResponse {
    BOOTSTRAP_CSS.get_request(headers.typed_get())
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
