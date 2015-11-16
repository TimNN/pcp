use libc;

use scoped_threadpool::Pool;

use std::sync::PoisonError;
use std::sync::{Mutex, MutexGuard};

pub fn stdin_isatty() -> bool {
    unsafe { libc::isatty(0) != 0 }
}

pub trait MutexExt {
    type TT;

    fn with_lock<R, F: FnOnce(&mut Self::TT) -> R>(&self, f: F) -> Result<R, PoisonError<MutexGuard<Self::TT>>>;
}

impl<T> MutexExt for Mutex<T> {
    type TT = T;

    fn with_lock<R, F: FnOnce(&mut T) -> R>(&self, f: F) -> Result<R, PoisonError<MutexGuard<T>>> {
        self.lock().map(|mut t| f(&mut *t))
    }
}

pub trait PoolExt {
    fn execute_on_all<F: Fn() + Send + Sync>(&mut self, f: F);
}

impl PoolExt for Pool {
    fn execute_on_all<F: Fn() + Send + Sync>(&mut self, f: F) {
        let thread_count = self.thread_count();

        self.scoped(|scope| {
            for _ in 0 .. thread_count {
                scope.execute(&f);
            }
        })
    }
}
