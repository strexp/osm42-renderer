use crate::config::AppConfig;
use bytes::Bytes;
use maplibre_native::{ImageRenderer, ImageRendererBuilder, Tile};
use std::{
    collections::HashMap,
    io::Cursor,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};
use tokio::sync::{mpsc, oneshot};

pub struct RenderJob {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub style_url: String,
    pub reply_to: oneshot::Sender<Result<Bytes, String>>,
}

pub struct RenderService {
    txs: Vec<mpsc::Sender<RenderJob>>,
    worker_handles: Vec<Option<thread::JoinHandle<()>>>, 
    next_worker: AtomicUsize,
}

impl RenderService {
    pub fn new(config: &AppConfig) -> Self {
        let num_workers = config.render_num_workers;
        let mut txs = Vec::with_capacity(num_workers);
        let mut worker_handles = Vec::with_capacity(num_workers);

        tracing::info!("Initializing RenderService with {} workers", num_workers);

        for i in 0..num_workers {
            let (tx, rx) = mpsc::channel(config.render_worker_queue_size);
            
            let handle = thread::Builder::new()
                .name(format!("ml-render-thread-{}", i))
                .stack_size(config.render_stack_size)
                .spawn(move || render_worker(rx))
                .unwrap();
                
            txs.push(tx);
            worker_handles.push(Some(handle));
        }
            
        Self { 
            txs, 
            worker_handles,
            next_worker: AtomicUsize::new(0),
        }
    }

    pub async fn submit_job(
        &self,
        _style_name: &str, 
        style_url: String,
        z: u8,
        x: u32,
        y: u32,
    ) -> Result<Bytes, String> {
        let (reply_to, rx) = oneshot::channel();
        let job = RenderJob { z, x, y, style_url, reply_to };

        if self.txs.is_empty() {
            return Err("RenderService is shutting down".to_string());
        }

        // Round-Robin
        let worker_idx = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.txs.len();

        self.txs[worker_idx]
            .send(job)
            .await
            .map_err(|_| "Render thread queue is full or unavailable".to_string())?;

        rx.await.map_err(|_| "Render thread crashed".to_string())?
    }
}

impl Drop for RenderService {
    fn drop(&mut self) {
        self.txs.clear();

        for (i, handle_opt) in self.worker_handles.iter_mut().enumerate() {
            if let Some(handle) = handle_opt.take() {
                tracing::debug!("Waiting for ml-render-thread-{} to shutdown cleanly...", i);
                let _ = handle.join();
                tracing::debug!("ml-render-thread-{} shutdown successfully.", i);
            }
        }
    }
}

fn render_worker(mut rx: mpsc::Receiver<RenderJob>) {
    let mut renderers: HashMap<String, ImageRenderer<Tile>> = HashMap::new();

    while let Some(job) = rx.blocking_recv() {
        let renderer = renderers.entry(job.style_url.clone()).or_insert_with(|| {
            tracing::debug!("Initializing MapLibre renderer for style: {}", job.style_url);
            
            let mut r = ImageRendererBuilder::default()
                .build_tile_renderer();
                
            if let Ok(parsed_url) = url::Url::parse(&job.style_url) {
                r.load_style_from_url(&parsed_url);
            }
            r
        });

        let result = render_and_encode(renderer, &job);
        let _ = job.reply_to.send(result);
    }
}

fn render_and_encode(
    renderer: &mut ImageRenderer<Tile>,
    job: &RenderJob,
) -> Result<Bytes, String> {
    let image = renderer
        .render_tile(job.z, job.x, job.y)
        .map_err(|e| format!("Rendering Error: {:?}", e))?;

    let mut png = Vec::new();
    image
        .as_image()
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| format!("Encode Error: {:?}", e))?;

    Ok(Bytes::from(png))
}
