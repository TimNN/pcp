use std::{mem, slice};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use chunk_vec::{Chunk, ChunkVec, ChunkWriter};
use config::{Config, IPair};
use pair::VPair;
use stats::{self, IterStats};
use util::{MutexExt, PoolExt};

use ctrlc::CtrlC;
use scoped_threadpool::Pool;

pub fn solve(config: &Config) {
    let iter = IterState::new(config);
    let mut pool = Pool::new(config.thread_cnt);
    let mut state = SolveState::new();
    let mut stats = IterStats::new();

    for _ in &iter {
        stats.iter(|| {
            pool.execute_on_all(|| {
                let mut writer = NodeWriter::new(&state);
                let mut apply_cnt = 0;
                let mut apply_success_cnt = 0;

                for chunk in &state {
                    for node in &chunk {
                        for pair in &*config.pairs {
                            apply_cnt += 1;

                            if let Some(new_node) = node.apply(&pair) {
                                apply_success_cnt += 1;

                                if new_node.pair.is_complete() {
                                    println!("success! n: {}, s: {}", new_node.depth, new_node.sum);
                                    iter.report_success();
                                }

                                writer.push(new_node);
                            }
                        }
                    }
                }

                stats::pairs_applied(apply_cnt);
                stats::pairs_successfully_applied(apply_success_cnt);
            })
        });

        state.swap();
    }

    if !iter.is_success() {
        println!("no success");
    }
}

struct SolveState {
    chunks: Mutex<ChunkVec<Node>>,
    next_working_set: Mutex<Vec<Chunk<Node>>>,
    current_working_set: Mutex<Vec<Chunk<Node>>>,
}

struct NodeWriter<'a> {
    state: &'a SolveState,
    inner: ChunkWriter<Node>,
}

struct NodeReader<'a> {
    state: &'a SolveState,
    inner: Chunk<Node>,
}

#[derive(Copy, Clone)]
struct Node {
    pair: VPair,
    sum: u16,
    depth: u16,
}

struct IterState {
    // Atomic so we can use for loops and access it's methods
    iter_cnt: AtomicUsize,
    iter_max: usize,
    success: AtomicBool,
    done: Arc<AtomicBool>,
}

impl SolveState {
    fn new() -> SolveState {
        let mut chunks = ChunkVec::new();
        let initial = chunks.get_with(Node::new());

        SolveState {
            chunks: Mutex::new(chunks),
            current_working_set: Mutex::new(vec![initial]),
            next_working_set: Mutex::new(vec![]),
        }
    }

    fn pop(&self) -> Option<Chunk<Node>> {
        self.current_working_set.with_lock(|cws| cws.pop()).unwrap()
    }

    fn push(&self, chunk: Chunk<Node>) {
        self.next_working_set.with_lock(|nws| nws.push(chunk)).unwrap();
    }

    fn new_chunk(&self) -> Chunk<Node> {
        self.chunks.with_lock(|c| c.get()).unwrap()
    }

    fn new_chunk_with(&self, node: Node) -> Chunk<Node> {
        self.chunks.with_lock(|c| c.get_with(node)).unwrap()
    }

    fn offer(&self, chunk: Chunk<Node>) {
        self.chunks.with_lock(|c| c.offer(chunk)).unwrap()
    }

    fn swap(&mut self) {
        let c = self.current_working_set.get_mut().unwrap();
        let n = self.next_working_set.get_mut().unwrap();

        mem::swap(c, n);
        assert!(n.is_empty());
    }
}

impl<'a> Iterator for &'a SolveState {
    type Item = NodeReader<'a>;

    fn next(&mut self) -> Option<NodeReader<'a>> {
        self.pop().map(|c| NodeReader::new(self, c))
    }
}

impl<'a> NodeWriter<'a> {
    fn new(state: &'a SolveState) -> NodeWriter<'a> {
        let inner = state.new_chunk().writer();

        NodeWriter {
            state: state,
            inner: inner,
        }
    }

    fn push(&mut self, node: Node) {
        match self.inner.push(node) {
            Ok(()) => (),
            Err(n) => {
                let mut tmp = self.state.new_chunk_with(n).writer();
                mem::swap(&mut tmp, &mut self.inner);
                self.state.push(tmp.into());
            }
        }
    }
}

impl<'a> Drop for NodeWriter<'a> {
    fn drop(&mut self) {
        unsafe {
            let mut tmp = mem::dropped();
            mem::swap(&mut tmp, &mut self.inner);
            self.state.push(tmp.into());
        }
    }
}

impl<'a> NodeReader<'a> {
    fn new(state: &'a SolveState, chunk: Chunk<Node>) -> NodeReader<'a> {
        NodeReader {
            state: state,
            inner: chunk,
        }
    }
}

impl<'a> Drop for NodeReader<'a> {
    fn drop(&mut self) {
        unsafe {
            let mut tmp = mem::dropped();
            mem::swap(&mut tmp, &mut self.inner);
            self.state.offer(tmp);
        }
    }
}

impl<'a, 'b> IntoIterator for &'b NodeReader<'a> {
    type Item = &'b Node;
    type IntoIter = slice::Iter<'b, Node>;

    fn into_iter(self) -> slice::Iter<'b, Node> {
        self.inner.iter()
    }
}

impl Node {
    fn new() -> Node {
        Node {
            pair: VPair::new(),
            sum: 0,
            depth: 0,
        }
    }

    fn apply(&self, pair: &IPair) -> Option<Node> {
        self.pair.apply(&pair.pair).map(|p| Node {
            pair: p,
            sum: self.sum + pair.sum_inc,
            depth: self.depth + pair.depth_inc,
        })
    }
}

impl IterState {
    fn new(config: &Config) -> IterState {
        let state = IterState {
            iter_cnt: AtomicUsize::new(0),
            iter_max: config.max_iter,
            success: AtomicBool::new(false),
            done: Arc::new(AtomicBool::new(false)),
        };

        let done_handle = Arc::downgrade(&state.done);

        CtrlC::set_handler(move || {
            let dh = done_handle.upgrade().expect("Ctrl+C pressed after handler was deregistered");
            println!("Interrupt received, aborting after current iteration.");
            dh.store(true, Ordering::Release);
        });

        state
    }

    fn report_success(&self) {
        self.success.store(true, Ordering::Release);
        self.done.store(true, Ordering::Release);
    }

    fn is_success(&self) -> bool {
        self.success.load(Ordering::Acquire)
    }
}

impl<'a> Iterator for &'a IterState {
    type Item = ();

    fn next(&mut self) -> Option<()> {
        if self.iter_cnt.load(Ordering::Acquire) < self.iter_max {
            self.iter_cnt.fetch_add(1, Ordering::AcqRel);

            if !self.done.load(Ordering::Acquire) {
                Some(())
            } else {
                None
            }
        } else {
            None
        }
    }
}
