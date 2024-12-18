// use std::thread;

// static mut COUNTER: usize = 1;

// fn main() {
//     let t1 = thread::spawn(move || {
//         unsafe { COUNTER += 10 };
//     });

//     let t2 = thread::spawn(move || {
//         unsafe { COUNTER *= 10 };
//     });

//     t2.join().unwrap();
//     t1.join().unwrap();

//     unsafe {
//         println!("COUNTER: {COUNTER}");
//     }
// }

// use std::{sync::atomic::{AtomicUsize, Ordering}, thread};

// static COUNTER: AtomicUsize = AtomicUsize::new(1);

// fn main() {
//   let t1 = thread::spawn(move || {
//     COUNTER.fetch_add(10, Ordering::SeqCst);
//   });

//   let t2 = thread::spawn(move || {
//     COUNTER.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| Some(v * 10)).unwrap();
//   });

//   t2.join().unwrap();
//   t1.join().unwrap();

//   println!("COUNTER: {}", COUNTER.load(Ordering::Relaxed));
// }

use std::{collections::HashMap, sync::Mutex, thread};

use lazy_static::lazy_static;

// 使用lazy_static初始化复杂的结构
lazy_static! {
  // 使用Mutex / RwLock来提供安全的并发写访问
  static ref STORE: Mutex<HashMap<&'static str, &'static [u8]>> = Mutex::new(HashMap::new());
}

fn main() {
    let t1 = thread::spawn(move || {
        let mut store = STORE.lock().unwrap();
        store.insert("hello", b"world");
    });

    let t2 = thread::spawn(move || {
        let mut store = STORE.lock().unwrap();
        store.insert("goodbye", b"world");
    });

    t2.join().unwrap();
    t1.join().unwrap();

    println!("store: {:?}", STORE.lock().unwrap());
}
