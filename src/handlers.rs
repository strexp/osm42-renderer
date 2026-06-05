use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{Json, Response},
};
use bytes::Bytes;
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc, time::SystemTime};
use tokio::fs;

use crate::state::AppState;

pub async fn style_json_handler(
    Path(style): Path<String>,
    headers: HeaderMap,
) -> Json<Value> {
    
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http");

    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:8080");
    
    let tile_url = format!("{}://{}/{}/{{z}}/{{x}}/{{y}}", scheme, host, style);

    Json(json!({
        "version": 8,
        "id": "blank-style",
        "sources": {
            "raster-tiles": {
                "type": "raster",
                "tiles": [
                    tile_url
                ],
                "tileSize": 512
            }
        },
        "layers": [
            {
                "id": "tiles",
                "type": "raster",
                "source": "raster-tiles",
                "minzoom": 0,
                "maxzoom": 19
            }
        ]
    }))
}

pub async fn catalog_handler(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(state.catalog_json.clone())
}

pub async fn tile_handler(
    Path((style, z, x, y)): Path<(String, u8, u32, u32)>,
    State(state): State<Arc<AppState>>,
) -> Result<Response, StatusCode> {
    
    if !state.valid_styles.contains(&style) {
        tracing::warn!("Requested style not found: {}", style);
        return Err(StatusCode::NOT_FOUND);
    }
    
    if state.config.disable_cache {
        let style_url = format!("{}/style/{}", state.config.style_base_url, style);
        match state.render_service.submit_job(&style, style_url, z, x, y).await {
            Ok(bytes) => {
                return Ok(Response::builder()
                    .header(header::CONTENT_TYPE, "image/png")
                    .header(header::CACHE_CONTROL, format!("max-age={}", state.config.cache_ttl.as_secs()))
                    .body(Body::from(bytes))
                    .unwrap());
            }
            Err(e) => {
                tracing::error!("Failed to serve tile (cache disabled): {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    let cache_key = format!("{}/{}/{}/{}", style, z, x, y);
    let state_clone = state.clone();
    let style_clone = style.clone();

    let tile_result = state.mem_cache.try_get_with(cache_key, async move {
        let relative_path = PathBuf::from(&style_clone)
            .join(z.to_string())
            .join(x.to_string())
            .join(format!("{}.png", y));
        let file_path = state_clone.config.cache_dir.join(&relative_path);

        if let Ok(metadata) = fs::metadata(&file_path).await {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = SystemTime::now().duration_since(modified) {
                    if age < state_clone.config.cache_ttl {
                        if let Ok(data) = fs::read(&file_path).await {
                            return Ok::<Bytes, String>(Bytes::from(data));
                        }
                    }
                }
            }
        }

        let style_url = format!("{}/style/{}", state_clone.config.style_base_url, style_clone);

        let rendered_bytes = state_clone
            .render_service
            .submit_job(&style_clone, style_url, z, x, y)
            .await?;

        let path_clone = file_path.clone();
        let bytes_clone = rendered_bytes.clone();
        tokio::spawn(async move {
            if let Some(parent) = path_clone.parent() {
                let _ = fs::create_dir_all(parent).await;
            }
            let _ = fs::write(path_clone, bytes_clone).await;
        });

        Ok(rendered_bytes)
    }).await;

    match tile_result {
        Ok(bytes) => Ok(Response::builder()
            .header(header::CONTENT_TYPE, "image/png")
            .header(
                header::CACHE_CONTROL,
                format!("public, max-age={}", state.config.cache_ttl.as_secs()),
            )
            .body(Body::from(bytes))
            .unwrap()),
        Err(e) => {
            tracing::error!("Failed to serve tile: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
