#[forbid(unsafe_code)]
use axum::{extract::Query, routing::get, Router};
use serde::Deserialize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new().route("/co2-ampel", get(receive_sensor_values));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
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

async fn receive_sensor_values(Query(params): Query<Params>) -> &'static str {
    tracing::debug!("data received: {params:?}");
    "done"
}
