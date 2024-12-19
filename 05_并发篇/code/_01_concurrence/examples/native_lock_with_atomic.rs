use core::fmt;
use std::{
    cell::RefCell,
    sync::{
        atomic::{self, AtomicBool, Ordering},
        Arc,
    },
    thread,
};

struct Lock<T> {
    locked: AtomicBool,
    data: RefCell<T>,
}

impl<T> fmt::Debug for Lock<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lock<{:?}>", self.data.borrow())
    }
}

// SAFETY: 我们确信Lock<T>很安全, 可以在多个线程中共享
unsafe impl<T> Sync for Lock<T> {}

impl<T> Lock<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: RefCell::new(data),
            locked: AtomicBool::new(false),
        }
    }

    pub fn lock(&self, op: impl FnOnce(&mut T)) {
        // 如果没拿到所就一值spin
        // while self
        //     .locked
        //     .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        //     .is_err()
        // {}

        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // 性能优化: compare_exchange需要独占访问, 当拿不到锁时, 我们先不停检测locked状态, 直到其unlocked后, 在尝试拿锁
            while self.locked.load(Ordering::Relaxed) == true {}
        }

        // 开始干活
        op(&mut self.data.borrow_mut()); // **3

        // 解锁
        self.locked.store(false, Ordering::Release);
    }
}

fn main() {
    let data = Arc::new(Lock::new(0));

    let data1 = data.clone();

    let t1 = thread::spawn(move || {
        data1.lock(|v| *v += 10);
    });

    let data2 = data.clone();
    let t2 = thread::spawn(move || {
        data2.lock(|v| *v *= 10);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("data: {:?}", data.data.borrow());
}
