use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex as StdMutex;
use std::collections::VecDeque;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio::time::Instant;

const RESTART_CAP_PER_HOUR: usize = 10;
const BACKOFF_BASE_SECS: u64 = 5;
const BACKOFF_MAX_SECS: u64 = 600;

#[derive(Clone)]
pub struct TaskHandle {
    pub name: String,
    pub running: bool,
    pub restart_count: u64,
    pub last_error: Option<String>,
    pub uptime_seconds: u64,
}

struct ManagedTask {
    name: String,
    handle: StdMutex<JoinHandle<()>>,
    factory: Box<dyn Fn() -> JoinHandle<()> + Send>,
    running: Arc<AtomicBool>,
    restart_count: AtomicU64,
    last_error: Arc<StdMutex<Option<String>>>,
    started_at: StdMutex<Instant>,
    restart_times: VecDeque<Instant>,
    next_restart_at: Instant,
    gave_up: bool,
}

pub struct TaskSupervisor {
    tasks: StdMutex<Vec<ManagedTask>>,
}

impl TaskSupervisor {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tasks: StdMutex::new(Vec::new()),
        })
    }

    pub async fn track<F>(self: &Arc<Self>, name: &str, factory: F)
    where
        F: Fn() -> JoinHandle<()> + Send + 'static,
    {
        let name = name.to_string();
        let running = Arc::new(AtomicBool::new(true));
        let last_error: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));

        let wrapped_factory = {
            let name = name.clone();
            let running = running.clone();
            let last_error = last_error.clone();
            Box::new(move || -> JoinHandle<()> {
                let name = name.clone();
                let running = running.clone();
                let last_error = last_error.clone();
                let handle = factory();
                let name2 = name.clone();
                tokio::spawn(async move {
                    // Wait for the inner task to complete
                    match handle.await {
                        Ok(()) => {
                            running.store(false, Ordering::Relaxed);
                            tracing::warn!("[Supervisor] Task '{}' completed unexpectedly", name);
                        }
                        Err(e) => {
                            running.store(false, Ordering::Relaxed);
                            let msg = if e.is_panic() {
                                format!("panic: {}", e)
                            } else {
                                format!("cancelled: {}", e)
                            };
                            tracing::error!("[Supervisor] Task '{}' failed: {}", name2, msg);
                            *last_error.lock().unwrap() = Some(msg);
                        }
                    }
                })
            })
        };

        let handle = wrapped_factory();

        let managed = ManagedTask {
            name: name.clone(),
            handle: StdMutex::new(handle),
            factory: wrapped_factory,
            running: running.clone(),
            restart_count: AtomicU64::new(0),
            last_error: last_error.clone(),
            started_at: StdMutex::new(Instant::now()),
            restart_times: VecDeque::new(),
            next_restart_at: Instant::now(),
            gave_up: false,
        };

        self.tasks.lock().unwrap().push(managed);
    }

    pub fn start_monitor(self: &Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let mut tasks = this.tasks.lock().unwrap();
                let now = Instant::now();
                for task in tasks.iter_mut() {
                    let is_finished = task.handle.lock().unwrap().is_finished();
                    if !is_finished {
                        continue;
                    }

                    if task.gave_up {
                        continue;
                    }

                    // Prune restart timestamps older than one hour.
                    while let Some(&t) = task.restart_times.front() {
                        if now.duration_since(t) > Duration::from_secs(3600) {
                            task.restart_times.pop_front();
                        } else {
                            break;
                        }
                    }

                    // Give up if we already restarted too many times this hour.
                    if task.restart_times.len() >= RESTART_CAP_PER_HOUR {
                        task.gave_up = true;
                        task.running.store(false, Ordering::Relaxed);
                        let msg = format!(
                            "exceeded {} restarts/hour — task halted",
                            RESTART_CAP_PER_HOUR
                        );
                        *task.last_error.lock().unwrap() = Some(msg.clone());
                        tracing::error!("[Supervisor] Task '{}': {}", task.name, msg);
                        continue;
                    }

                    // Exponential backoff: 5s, 10s, 20s, ... capped at 10min.
                    if now < task.next_restart_at {
                        continue;
                    }
                    let attempt = task.restart_times.len() + 1;
                    let exp = ((attempt - 1) as u32).min(7);
                    let delay_secs =
                        BACKOFF_BASE_SECS.saturating_mul(1u64 << exp).min(BACKOFF_MAX_SECS);

                    task.restart_count.fetch_add(1, Ordering::Relaxed);
                    let count = task.restart_count.load(Ordering::Relaxed);
                    tracing::warn!(
                        "[Supervisor] Restarting task '{}' (attempt #{}, backoff {}s)",
                        task.name, count, delay_secs
                    );
                    task.restart_times.push_back(now);
                    task.next_restart_at = now + Duration::from_secs(delay_secs);
                    task.running.store(true, Ordering::Relaxed);
                    *task.started_at.lock().unwrap() = Instant::now();
                    *task.handle.lock().unwrap() = (task.factory)();
                }
            }
        });
    }

    pub fn handles(&self) -> Vec<TaskHandle> {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .iter()
            .map(|t| {
                let started_at = *t.started_at.lock().unwrap();
                TaskHandle {
                    name: t.name.clone(),
                    running: t.running.load(Ordering::Relaxed),
                    restart_count: t.restart_count.load(Ordering::Relaxed),
                    last_error: t.last_error.lock().unwrap().clone(),
                    uptime_seconds: started_at.elapsed().as_secs(),
                }
            })
            .collect()
    }
}