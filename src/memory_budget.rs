use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct MemoryBudget {
    inner: Arc<(Mutex<usize>, Condvar)>,
    limit: usize,
}

impl MemoryBudget {
    pub fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new((Mutex::new(0), Condvar::new())),
            limit,
        }
    }

    pub fn acquire(&self, bytes: usize) {
        let (lock, cv) = &*self.inner;
        let mut used = lock.lock().unwrap();
        while *used + bytes > self.limit {
            used = cv.wait(used).unwrap();
        }
        *used += bytes;
    }

    pub fn release(&self, bytes: usize) {
        let (lock, cv) = &*self.inner;
        let mut used = lock.lock().unwrap();
        *used -= bytes;
        cv.notify_all();
    }
}