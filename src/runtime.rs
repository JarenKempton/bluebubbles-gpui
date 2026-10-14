use std::{future::Future, sync::OnceLock};

use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("bb-io")
            .enable_all()
            .build()
            .expect("failed to start tokio runtime")
    })
}

pub async fn run<F>(future: F) -> F::Output
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime()
        .spawn(future)
        .await
        .expect("background task panicked")
}

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    runtime().spawn(future);
}
