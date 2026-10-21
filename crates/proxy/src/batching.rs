//! Port of upstream `optillm/batching.py`: requests are queued per
//! `model:approach` key, collected into batches of up to `batch_size` within
//! `batch_wait_ms`, then processed (sequentially, as upstream does) by
//! `server::process_batch_item`. Failures fail the whole batch.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::server::{process_batch_item, AppState, Prepared};

pub struct BatchRequest {
    pub prepared: Prepared,
}

struct Queued {
    req: BatchRequest,
    reply: oneshot::Sender<Result<Value, String>>,
}

pub struct Batcher {
    max_batch_size: usize,
    max_wait: Duration,
    queues: Mutex<HashMap<String, mpsc::UnboundedSender<Queued>>>,
    state: Mutex<Weak<AppState>>,
}

impl Batcher {
    pub fn new(max_batch_size: usize, max_wait_ms: u64) -> Self {
        tracing::info!(
            "RequestBatcher initialized: max_batch_size={}, max_wait_ms={}",
            max_batch_size,
            max_wait_ms
        );
        Self {
            max_batch_size,
            max_wait: Duration::from_millis(max_wait_ms),
            queues: Mutex::new(HashMap::new()),
            state: Mutex::new(Weak::new()),
        }
    }

    /// The processor needs the server state (`set_processor` upstream).
    pub fn set_state(&self, state: &Arc<AppState>) {
        *self.state.lock().unwrap() = Arc::downgrade(state);
    }

    /// `RequestBatcher.add_request`.
    pub async fn add_request(self: &Arc<Self>, req: BatchRequest) -> Result<Value, String> {
        let wrap = |e: String| format!("Failed to process request: {}", e);
        if req.prepared.stream {
            return Err(wrap("Streaming requests cannot be batched".into()));
        }
        let key = format!("{}:{}", req.prepared.model, req.prepared.optillm_approach);
        let sender = {
            let mut queues = self.queues.lock().unwrap();
            queues
                .entry(key)
                .or_insert_with(|| {
                    let (tx, rx) = mpsc::unbounded_channel();
                    let me = self.clone();
                    tokio::spawn(async move { me.batch_processor(rx).await });
                    tx
                })
                .clone()
        };
        let (reply, wait) = oneshot::channel();
        sender
            .send(Queued { req, reply })
            .map_err(|_| wrap("batch processor stopped".into()))?;
        match wait.await {
            Ok(r) => r.map_err(wrap),
            Err(_) => Err(wrap("batch processor dropped the request".into())),
        }
    }

    async fn batch_processor(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<Queued>) {
        loop {
            let first = match rx.recv().await {
                Some(q) => q,
                None => return,
            };
            let deadline = Instant::now() + self.max_wait;
            let mut batch = vec![first];
            while batch.len() < self.max_batch_size {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                match tokio::time::timeout(deadline - now, rx.recv()).await {
                    Ok(Some(q)) => batch.push(q),
                    _ => break,
                }
            }
            tracing::info!("Processing batch of {} requests", batch.len());
            self.process_batch(batch).await;
        }
    }

    async fn process_batch(&self, batch: Vec<Queued>) {
        let state = self.state.lock().unwrap().upgrade();
        let Some(state) = state else {
            for q in batch {
                let _ = q.reply.send(Err(
                    "Batch processing failed: No batch processor function set".into(),
                ));
            }
            return;
        };
        let mut responses = Vec::new();
        let mut failure: Option<String> = None;
        for (i, q) in batch.iter().enumerate() {
            match process_batch_item(&state, &q.req.prepared, i).await {
                Ok(v) => responses.push(v),
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        match failure {
            Some(e) => {
                let msg = format!("Batch processing failed: {}", e);
                tracing::error!("{}", msg);
                for q in batch {
                    let _ = q.reply.send(Err(msg.clone()));
                }
            }
            None => {
                for (q, r) in batch.into_iter().zip(responses) {
                    let _ = q.reply.send(Ok(r));
                }
            }
        }
    }
}
