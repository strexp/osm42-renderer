use clap::Parser;
use std::{path::PathBuf, time::Duration};

#[derive(Parser, Debug)]
#[command(name = "osm42-renderer", version, about = "Raster tile renderer for OSM42")]
pub struct Cli {
    #[arg(long, env = "BIND_IP", default_value = "[::]")]
    pub bind_ip: String,

    #[arg(long, env = "PORT", default_value_t = 8080)]
    pub bind_port: u16,

    #[arg(long, env = "STYLE_BASE_URL", default_value = "http://localhost:8080")]
    pub style_base_url: String,

    #[arg(long, env = "CACHE_DIR", default_value = "./tiles_cache")]
    pub cache_dir: PathBuf,

    #[arg(long = "cache-ttl", env = "CACHE_TTL_SECONDS", default_value_t = 604800)]
    pub cache_ttl_secs: u64,

    #[arg(long)]
    pub benchmark: bool,

    #[arg(long, env = "DISABLE_CACHE", default_value_t = false)]
    pub disable_cache: bool,

    #[arg(long, env = "BENCHMARK_CONCURRENCY", default_value_t = 50)]
    pub benchmark_concurrency: usize,

    #[arg(long, env = "BENCHMARK_DURATION_SECS", default_value_t = 10)]
    pub benchmark_duration_secs: u64,

    #[arg(long, env = "RENDER_WORKER_QUEUE_SIZE", default_value_t = 1024)]
    pub render_worker_queue_size: usize,

    #[arg(long, env = "RENDER_NUM_WORKERS")]
    pub render_num_workers: Option<usize>,

    #[arg(long, env = "RENDER_STACK_SIZE", default_value_t = 8)]
    pub render_stack_size_mb: usize,

    #[arg(long, env = "RENDER_IMAGE_SIZE", default_value_t = 512)]
    pub render_image_size: u32,
}

#[derive(Clone)]
pub struct AppConfig {
    pub cache_dir: PathBuf,
    pub cache_ttl: Duration,
    pub style_base_url: String,
    pub listen_addr: String,
    pub benchmark: bool,
    pub disable_cache: bool,
    pub benchmark_concurrency: usize,
    pub benchmark_duration_secs: u64,
    pub render_worker_queue_size: usize,
    pub render_num_workers: usize,
    pub render_stack_size: usize,
    pub render_image_size: u32,
}

impl AppConfig {
    pub fn load() -> Self {
        let cli = Cli::parse();
        
        let render_num_workers = cli.render_num_workers.unwrap_or_else(|| {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        });

        Self {
            cache_dir: cli.cache_dir,
            cache_ttl: Duration::from_secs(cli.cache_ttl_secs),
            style_base_url: cli.style_base_url,
            listen_addr: format!("{}:{}", cli.bind_ip, cli.bind_port),
            benchmark: cli.benchmark,
            disable_cache: cli.disable_cache,
            benchmark_concurrency: cli.benchmark_concurrency,
            benchmark_duration_secs: cli.benchmark_duration_secs,
            render_worker_queue_size: cli.render_worker_queue_size,
            render_num_workers,
            render_stack_size: cli.render_stack_size_mb * 1024 * 1024,
            render_image_size: cli.render_image_size,
        }
    }
}
