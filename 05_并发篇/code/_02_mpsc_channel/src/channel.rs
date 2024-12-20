use anyhow::{anyhow, Ok, Result};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
};

/// 发送者和接收者之间共享一个VecDequeue, 用Mutex互斥, 用Condvar通知
/// 同时还要记录有多少个senders和receivers
pub struct Shared<T> {
    queue: Mutex<VecDeque<T>>,
    available: Condvar,
    senders: AtomicUsize,
    receivers: AtomicUsize,
}

const INITIAL_SIZE: usize = 32;
impl<T> Default for Shared<T> {
    fn default() -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(INITIAL_SIZE)),
            available: Condvar::new(),
            senders: AtomicUsize::new(1),
            receivers: AtomicUsize::new(1),
        }
    }
}

/// 发送者
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn send(&mut self, t: T) -> Result<()> {
        // 如果没有消费者, 写入时出错
        if self.total_receivers() == 0 {
            return Err(anyhow!("no receiver left"));
        }

        // 加锁, 访问VecDequeue, 压入数据, 然后立即释放锁
        let was_empty = {
            let mut inner = self.shared.queue.lock().unwrap();
            let empty = inner.is_empty();
            inner.push_back(t);
            empty
        };

        // 通知任意一个被挂起等待的消费者有数据
        if was_empty {
            self.shared.available.notify_one();
        }

        Ok(())
    }

    pub fn total_receivers(&self) -> usize {
        self.shared.receivers.load(Ordering::SeqCst)
    }

    pub fn total_queued_items(&self) -> usize {
        self.shared.queue.lock().unwrap().len()
    }
}

/// 克隆sender
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.shared.senders.fetch_add(1, Ordering::AcqRel);

        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

/// Drop sender
impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let old = self.shared.senders.fetch_sub(1, Ordering::AcqRel);
        // sender走光了, 唤醒receiver读取数据(如果队列中还有的话), 读不到就出错
        if old <= 1 {
            // 因为我们实现的是MPSC, receiver只有一个, 所以notify_all等价于notify_one
            self.shared.available.notify_all();
        }
    }
}

/// 接收者
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
    cache: VecDeque<T>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Result<T> {
        // 无锁 fast path
        if let Some(t) = self.cache.pop_front() {
            return Ok(t);
        }

        // 拿到队列的锁
        let mut inner = self.shared.queue.lock().unwrap();
        loop {
            match inner.pop_front() {
                // 读到数据返回, 锁就释放
                Some(t) => {
                    // 如果当前队列中还有数据, 那么就把消费者自身缓存的队列和共享队列swap
                    // 这样以后在读取, 就可以从self.queue中无锁读取
                    if !inner.is_empty() {
                        std::mem::swap(&mut self.cache, &mut inner);
                    }

                    return Ok(t);
                }
                // 读不到数据, 且已经没有生产者, 释放锁并返回错误
                None if self.total_senders() == 0 => return Err(anyhow!("no sender left")),
                // 读不到锁, 把锁提交给available Condvar, 它会释放瘦ing挂起线程, 等待notify
                None => {
                    // 当Condvar被唤醒后返回MutexGuard, 我们可以loop回去拿数据
                    inner = self
                        .shared
                        .available
                        .wait(inner)
                        .map_err(|_| anyhow!("lock poisoned"))?;
                }
            }
        }
    }

    pub fn total_senders(&self) -> usize {
        self.shared.senders.load(Ordering::SeqCst)
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.receivers.fetch_sub(1, Ordering::AcqRel);
    }
}

impl<T> Iterator for Receiver<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.recv().ok()
    }
}

/// 创建一个unbounded channel
pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Shared::default();
    let shared = Arc::new(shared);
    (
        Sender {
            shared: shared.clone(),
        },
        Receiver {
            shared,
            cache: VecDeque::with_capacity(INITIAL_SIZE),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::{process::id, thread, time::Duration};

    use super::*;

    #[test]
    fn channel_should_work() {
        let (mut s, mut r) = unbounded();
        s.send("hello world!".to_string()).unwrap();
        let msg = r.recv().unwrap();
        assert_eq!(msg, "hello world!");
    }

    #[test]
    fn multiple_senders_should_work() {
        let (mut s, mut r) = unbounded();
        let mut s1 = s.clone();
        let mut s2 = s.clone();
        let t = thread::spawn(move || {
            s.send(1).unwrap();
        });

        let t1 = thread::spawn(move || {
            s1.send(2).unwrap();
        });

        let t2 = thread::spawn(move || {
            s2.send(3).unwrap();
        });

        for handle in [t, t1, t2] {
            handle.join().unwrap();
        }

        let mut result = vec![r.recv().unwrap(), r.recv().unwrap(), r.recv().unwrap()];

        // 在这个测试数据里, 数据到达的顺序是不确定的 , 所以我们在排个序在assert
        result.sort();

        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn receiver_should_be_block_nothing_to_read() {
        let (mut s, r) = unbounded();
        let mut s1 = s.clone();
        thread::spawn(move || {
            for (idx, i) in r.into_iter().enumerate() {
                // 如果读到数据, 确保它和发送的数据一致
                assert_eq!(idx, i);
            }

            // 读不到应该休眠, 所以不会执行这一句, 执行到这一句说明逻辑出错
            assert!(false);
        });

        thread::spawn(move || {
            for i in 0..100_usize {
                s.send(i).unwrap();
            }
        });

        // 1ms足够让生产者发完100个消息, 消费者消费完100消息并阻塞
        thread::sleep(Duration::from_millis(1));

        // 再次发送数据, 唤醒消费者
        for i in 100..200_usize {
            s1.send(i).unwrap();
        }

        // 留点事件让receiver处理
        thread::sleep(Duration::from_millis(1));

        // 如果receiver被正常唤醒处理, 那么队列里的数据会被读完
        assert_eq!(s1.total_queued_items(), 0);
    }

    #[test]
    fn last_sender_drop_should_err_when_receive() {
        let (s, mut r) = unbounded();
        let s1 = s.clone();
        let senders = [s, s1];
        let total = senders.len();

        // sender即用即抛
        for mut sender in senders {
            thread::spawn(move || {
                sender.send("hello").unwrap();
            })
            .join()
            .unwrap();
        }

        // 虽然没有sender了, 接收者依然可以接收已经在队列里的数据
        for _ in 0..total {
            r.recv().unwrap();
        }

        // 然而, 读取更多的数据会出错
        assert!(r.recv().is_err());
    }

    #[test]
    fn receiver_drop_should_err_when_send() {
        let (mut s1, mut s2) = {
            let (s, _) = unbounded();
            let s1 = s.clone();
            let s2 = s.clone();
            (s1, s2)
        };

        assert!(s1.send(1).is_err());
        assert!(s2.send(1).is_err());
    }

    #[test]
    fn receiver_shall_be_notified_when_all_senders_exit() {
        let (s, mut r) = unbounded::<usize>();
        // 用于两个线程同步
        let (mut sender, mut receiver) = unbounded::<usize>();

        let t1 = thread::spawn(move || {
            // 保证r.recv()咸鱼t2的drop执行
            sender.send(0).unwrap();
            assert!(r.recv().is_err());
        });

        thread::spawn(move || {
            receiver.recv().unwrap();
            drop(s);
        });

        t1.join().unwrap();
    }

    #[test]
    fn channel_fast_path_should_work() {
        let (mut s, mut r) = unbounded();
        for i in 0..10_usize {
            s.send(i).unwrap();
        }

        assert!(r.cache.is_empty());

        // 读取一个数据, 此时应该会导致swap, cache中有数据
        assert_eq!(0, r.recv().unwrap());

        // 还有9个数据在cache中
        assert_eq!(r.cache.len(), 9);

        // 在queue中没有数据了
        assert_eq!(s.total_queued_items(), 0);

        // 从cache中读取剩下的数据
        for (idx, i) in r.into_iter().take(9).enumerate() {
            assert_eq!(idx + 1, i);
        }
    }
}
