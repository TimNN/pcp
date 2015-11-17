use alloc::{self, heap};
use std::{mem, ops, ptr, slice};
use std::ptr::Unique;

use stats;

pub const CHUNK_MB: usize = 64;

const MB: usize = 1024 * 1024;
const CHUNK_SIZE: usize = CHUNK_MB * MB;

pub struct Chunk<T: Copy> {
    ptr: Unique<T>,
    used: usize,
}

pub struct ChunkWriter<T: Copy> {
    chunk: Chunk<T>,
    next: *mut T,
}

pub struct ChunkVec<T: Copy> {
    chunks: Vec<Chunk<T>>,
}

impl<T: Copy> Chunk<T> {
    fn new() -> Chunk<T> {
        assert!(mem::size_of::<T>() > 0, "zero sized types are not supported");

        stats::chunk_allocated();

        unsafe {
            let ptr = heap::allocate(CHUNK_SIZE, mem::align_of::<T>());

            if ptr.is_null() {
                alloc::oom();
            }

            Chunk {
                ptr: Unique::new(ptr as *mut _),
                used: 0,
            }
        }
    }

    pub fn clear(&mut self) {
        self.used = 0;
    }

    pub fn writer(self) -> ChunkWriter<T> {
        let next = unsafe { self.ptr.offset(self.used as isize) };

        ChunkWriter {
            chunk: self,
            next: next,
        }
    }

    fn prepare_push(&mut self) -> Result<(), ()> {
        if self.used < Self::cap() {
            self.used += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    fn cap() -> usize {
        CHUNK_SIZE / mem::size_of::<T>()
    }
}

impl <T: Copy> Drop for Chunk<T> {
    fn drop(&mut self) {
        stats::chunk_deallocated();

        unsafe {
            heap::deallocate(*self.ptr as *mut _, CHUNK_SIZE, mem::align_of::<T>());
        }
    }
}

impl<T: Copy> ChunkWriter<T> {
    pub fn push(&mut self, val: T) -> Result<(), T> {
        match self.chunk.prepare_push() {
            Ok(_) => unsafe {
                ptr::write(self.next, val);
                self.next = self.next.offset(1);
                Ok(())
            },
            Err(_) => Err(val),
        }
    }
}

impl<T: Copy> ChunkVec<T> {
    pub fn new() -> ChunkVec<T> {
        ChunkVec {
            chunks: Vec::new(),
        }
    }

    pub fn get(&mut self) -> Chunk<T> {
        match self.chunks.pop() {
            Some(chunk) => chunk,
            None => Chunk::new(),
        }
    }

    pub fn get_with(&mut self, val: T) -> Chunk<T> {
        let mut chunk = self.get();

        chunk.used += 1;
        unsafe {
            ptr::write(*chunk.ptr, val);
        }

        chunk
    }

    pub fn offer(&mut self, mut chunk: Chunk<T>) {
        chunk.clear();
        self.chunks.push(chunk);
    }
}

impl<T: Copy> From<ChunkWriter<T>> for Chunk<T> {
    fn from(w: ChunkWriter<T>) -> Chunk<T> {
        w.chunk
    }
}

impl<T: Copy> ops::Deref for Chunk<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(*self.ptr, self.used)
        }
    }
}

impl<T: Copy> ops::DerefMut for Chunk<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(*self.ptr, self.used)
        }
    }
}
