use libc;

use scoped_threadpool::Pool;

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::sync::PoisonError;
use std::sync::{Mutex, MutexGuard};

use binary_heap_by::{BinaryHeapBy, CmpFn};

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

pub struct MultiMergeIter<'a, I: 'a + Iterator, F: CmpFn<I::Item>> {
    iters: BinaryHeapBy<(I::Item, &'a mut I), Tuple0Cmp<I::Item, F>>,
}

struct Tuple0Cmp<T, F: CmpFn<T>>(PhantomData<(T, F)>);

impl<T, S, F: CmpFn<T>> CmpFn<(T, S)> for Tuple0Cmp<T, F> {
    fn cmp(a: &(T, S), b: &(T, S)) -> Ordering {
        F::cmp(&a.0, &b.0)
    }
}

impl<'a, I: 'a + Iterator, F: CmpFn<I::Item>> MultiMergeIter<'a, I, F> {
    pub fn new(iters: &'a mut [I]) -> MultiMergeIter<'a, I, F> {
        let mut its = BinaryHeapBy::with_capacity_by(iters.len());

        for iter in iters.iter_mut() {
            if let Some(el) = iter.next() {
                its.push((el, iter))
            }
        }

        MultiMergeIter {
            iters: its,
        }
    }
}

impl<'a, I: 'a + Iterator, F: CmpFn<I::Item>> Iterator for MultiMergeIter<'a, I, F> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iters.pop().map(|t| {
            let this = t.0;

            if let Some(next) = t.1.next() {
                self.iters.push((next, t.1))
            }

            this
        })
    }
}
