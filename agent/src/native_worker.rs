//! Ownership and bounded shutdown for native threads that cooperate with Stop.
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct NativeWorker {
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl NativeWorker {
    pub fn spawn(work: impl FnOnce(Arc<AtomicBool>) + Send + 'static) -> Self {
        let stopping = Arc::new(AtomicBool::new(false));
        let signal = stopping.clone();
        Self { stopping, thread: Some(std::thread::spawn(move || work(signal))) }
    }
    pub async fn shutdown(mut self) -> Result<(), &'static str> {
        self.stopping.store(true, Ordering::Release);
        let finished = tokio::time::timeout(Duration::from_secs(2), async {
            while !self.thread.as_ref().unwrap().is_finished() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await;
        if finished.is_err() { return Err("Native worker did not stop within two seconds"); }
        self.thread.take().unwrap().join().map_err(|_| "Native worker panicked")
    }
}
impl Drop for NativeWorker {
    fn drop(&mut self) { self.stopping.store(true, Ordering::Release); }
}

#[cfg(test)]
mod native_lifecycle_tests {
    use super::*;
    #[tokio::test]
    async fn shutdown_waits_for_native_cleanup_and_drop_requests_stop() {
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = NativeWorker::spawn(move |stop| {
            while !stop.load(Ordering::Acquire) { std::thread::sleep(Duration::from_millis(1)); }
            tx.send(()).unwrap();
        });
        worker.shutdown().await.unwrap();
        rx.try_recv().expect("Shutdown must wait for cleanup");
        let (tx, rx) = tokio::sync::oneshot::channel();
        let worker = NativeWorker::spawn(move |stop| {
            while !stop.load(Ordering::Acquire) { std::thread::sleep(Duration::from_millis(1)); }
            let _ = tx.send(());
        });
        drop(worker);
        tokio::time::timeout(Duration::from_secs(2), rx).await.unwrap().unwrap();
    }
}
