use libc;

use scoped_threadpool::Pool;

use std::cmp::Ordering;
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

    fn process_each<T: Send, I: IntoIterator<Item=T>, F: Fn(T) + Send + Sync>(&mut self, i: I, f: F);
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

    fn process_each<T: Send, I: IntoIterator<Item=T>, F: Fn(T) + Send + Sync>(&mut self, i: I, f: F) {
        self.scoped(|scope| {
            for v in i {
                scope.execute(|| f(v))
            }
        })
    }
}

pub trait OrderingExt {
    fn then<F: FnOnce() -> Ordering>(self, f: F) -> Ordering;
}

impl OrderingExt for Ordering {
    fn then<F: FnOnce() -> Ordering>(self, f: F) -> Ordering {
        match self {
            Ordering::Equal => f(),
            _ => self,
        }
    }
}
