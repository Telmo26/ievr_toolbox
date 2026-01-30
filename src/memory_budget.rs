use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug, Default)]
struct State {
    used: usize,
    waiting_decompression: usize,
    reserved_for_decompression: usize
}

#[derive(Clone)]
pub struct MemoryPool {
    inner: Arc<(Mutex<State>, Condvar)>,
    limit: usize,
}

impl MemoryPool {
    pub fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new((Mutex::new(State::default()), Condvar::new())),
            limit,
        }
    }

    pub fn acquire_decompression(&self, bytes: usize) {
        let (lock, cv) = &*self.inner;
        let mut state = lock.lock().unwrap();

        state.waiting_decompression += 1;
        state.reserved_for_decompression = state.reserved_for_decompression.max(bytes);

        while state.used + bytes > self.limit {
            state = cv.wait(state).unwrap();
        }

        state.waiting_decompression -= 1;
        if state.waiting_decompression == 0 {
            state.reserved_for_decompression = 0;
        }
        state.used += bytes;
    }

    pub fn acquire_decryption(&self, bytes: usize) {
        let (lock, cv) = &*self.inner;
        let mut state = lock.lock().unwrap();

        while state.used + bytes > self.limit || state.used + bytes + state.reserved_for_decompression > self.limit {
            state = cv.wait(state).unwrap();
        }

        state.used += bytes;
    }

    pub fn release(&self, bytes: usize) {
        let (lock, cv) = &*self.inner;
        let mut state = lock.lock().unwrap();
        
        state.used -= bytes;
        
        cv.notify_all();
    }

    pub fn limit(&self) -> usize {
        self.limit
    }
}