use crate::{config::AppConfig, render::RenderService};
use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::task;

pub async fn run_benchmark(config: AppConfig, mut styles_list: Vec<String>) {
    tracing::info!("Starting benchmark mode...");
    
    if styles_list.is_empty() {
        styles_list.push("default".to_string());
        tracing::warn!("Empty style list, using fallback: default");
    } else {
        tracing::info!("Loaded styles for benchmark: {:?}", styles_list);
    }

    tracing::info!("Initializing RenderService...");
    
    let render_service = Arc::new(RenderService::new(&config));
    
    let concurrency = config.benchmark_concurrency;           
    let duration_secs = config.benchmark_duration_secs;         
    let duration = Duration::from_secs(duration_secs);

    tracing::info!(
        "Benchmarking: Styles Count = {}, Concurrency = {}, Duration = {}s",
        styles_list.len(),
        concurrency,
        duration_secs
    );

    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let error_counter = Arc::new(AtomicUsize::new(0));

    let mut tasks = Vec::with_capacity(concurrency);
    let styles_arc = Arc::new(styles_list);

    for _ in 0..concurrency {
        let pool = render_service.clone();
        let counter = counter.clone();
        let error_counter = error_counter.clone();
        let base_url = config.style_base_url.clone();
        let styles = styles_arc.clone();

        tasks.push(task::spawn(async move {
            let end_time = start_time + duration;
            
            while Instant::now() < end_time {
                let (z, x, y, style_name) = {
                    let mut rng = rand::thread_rng();
                    let z = rng.gen_range(0..=14);
                    let max_val = 1 << z;
                    let x = rng.gen_range(0..max_val);
                    let y = rng.gen_range(0..max_val);
                    
                    let style_idx = rng.gen_range(0..styles.len());
                    let style_name = styles[style_idx].clone();
                    
                    (z, x, y, style_name)
                };
                
                let style_url = format!("{}/style/{}", base_url, style_name);
                
                match pool.submit_job(&style_name, style_url, z, x, y).await {
                    Ok(_) => {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        error_counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for t in tasks {
        let _ = t.await;
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let total_success = counter.load(Ordering::Relaxed);
    let total_errors = error_counter.load(Ordering::Relaxed);
    let tps = total_success as f64 / elapsed;

    println!("\n========================================");
    println!(" Benchmark Results");
    println!("========================================");
    println!(" Elapsed Time   : {:.2} s", elapsed);
    println!(" Total Success  : {}", total_success);
    println!(" Total Errors   : {}", total_errors);
    println!(" Average TPS    : {:.2} tiles/sec", tps);
    println!("========================================\n");
}
