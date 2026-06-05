mod benchmark;
mod cache;
mod config;
mod handlers;
mod render;
mod state;

use axum::{routing::get, Router};
use dotenvy::dotenv;
use moka::future::Cache;
use serde_json::Value;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    config::AppConfig, 
    handlers::{catalog_handler, tile_handler, style_json_handler}, 
    render::RenderService, 
    state::AppState
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let _ = dotenv();

    let config = AppConfig::load();

    tracing::info!("Fetching catalog from {}...", config.style_base_url);
    let catalog_url = format!("{}/catalog", config.style_base_url);
    
    let catalog_json: Value = match reqwest::get(&catalog_url).await {
        Ok(resp) => resp.json().await.unwrap_or_else(|e| {
            tracing::error!("Failed to parse catalog JSON: {}", e);
            serde_json::json!({ "styles": {} })
        }),
        Err(e) => {
            tracing::error!("Failed to fetch catalog: {}", e);
            serde_json::json!({ "styles": {} })
        }
    };

    let mut valid_styles = HashSet::new();
    if let Some(styles) = catalog_json.get("styles").and_then(|s| s.as_object()) {
        for style_name in styles.keys() {
            valid_styles.insert(style_name.clone());
        }
    }
    tracing::info!("Loaded {} valid styles from upstream", valid_styles.len());

    if config.benchmark {
        let styles_list: Vec<String> = valid_styles.into_iter().collect();
        benchmark::run_benchmark(config, styles_list).await;
        return;
    }

    if !config.disable_cache {
        tokio::fs::create_dir_all(&config.cache_dir).await.unwrap();

        let prune_dir = config.cache_dir.clone();
        let ttl = config.cache_ttl;
        tokio::task::spawn_blocking(move || loop {
            std::thread::sleep(Duration::from_secs(3600));
            cache::prune_expired_cache_blocking(&prune_dir, ttl);
        });
    } else {
        tracing::warn!("Cache DISABLED. All requests will be rendered directly.");
    }

    let render_service = RenderService::new(&config);
    tracing::info!("Initialized MapLibre RenderService");

    let mem_cache = Cache::builder()
        .max_capacity(50_000)
        .time_to_idle(Duration::from_secs(120))
        .build();

    let state = Arc::new(AppState {
        config: config.clone(),
        mem_cache,
        render_service,
        valid_styles,
        catalog_json,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/catalog", get(catalog_handler))
        .route("/{style}/{z}/{x}/{y}", get(tile_handler))
        .route("/style/{style}", get(style_json_handler))
        .with_state(state)
        .layer(cors);

    let listener = TcpListener::bind(&config.listen_addr).await.unwrap();
    tracing::info!("Tile Server listening on {}", config.listen_addr);

    axum::serve(listener, app).await.unwrap();
}
