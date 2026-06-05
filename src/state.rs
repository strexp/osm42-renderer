use bytes::Bytes;
use moka::future::Cache;
use serde_json::Value;
use std::collections::HashSet;

use crate::{config::AppConfig, render::RenderService};

pub struct AppState {
    pub config: AppConfig,
    pub mem_cache: Cache<String, Bytes>, 
    pub render_service: RenderService,
    pub valid_styles: HashSet<String>,
    pub catalog_json: Value,
}
