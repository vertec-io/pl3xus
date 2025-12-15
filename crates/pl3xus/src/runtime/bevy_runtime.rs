use crate::Runtime;
use std::future::Future;

use super::JoinHandle;

impl Runtime for bevy::tasks::TaskPool {
    type JoinHandle = Option<bevy::tasks::Task<()>>;

    fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) -> Self::JoinHandle {
        #[cfg(not(target_arch = "wasm32"))]
        {
            tracing::debug!("[TaskPool::spawn] Spawning and detaching task");
            let task = self.spawn(task);
            task.detach();
            tracing::debug!("[TaskPool::spawn] Task detached");
            None
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.spawn(task);
            None
        }
    }

    fn spawn_local(&self, task: impl Future<Output = ()> + 'static) -> Self::JoinHandle {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let task = self.spawn_local(task);
            task.detach();
            None
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.spawn_local(task);
            None
        }
    }
}

impl JoinHandle for Option<bevy::tasks::Task<()>> {
    fn abort(&mut self) {
        self.take();
    }
}
