// use std::alloc::{GlobalAlloc, Layout, System};

// struct MyAllocator;

// unsafe impl GlobalAlloc for MyAllocator {
//     unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//         let data = System.alloc(layout);
//         eprintln!("ALLOC: {:?}, size {}", data, layout.size());
//         data
//     }

//     unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//         System.dealloc(ptr, layout);
//         eprintln!("FREE: {:?}, size {}", ptr, layout.size());
//     }
// }

// #[global_allocator]
// static GLOBAL: MyAllocator = MyAllocator;

// #[allow(dead_code)]
// struct Matrix {
//     data: [u8; 505],
// }

// impl Default for Matrix {
//     fn default() -> Self {
//         Self { data: [0; 505] }
//     }
// }

// fn main() {
//     // 在这句之前已经有好多内存分配
//     let data = Box::new(Matrix::default());

//     // 输出中有一个1024大小的内存分配, 是println!导致的
//     println!(
//         "!!! allocated memory: {:p}, len: {}",
//         &*data,
//         std::mem::size_of::<Matrix>()
//     )

//     // data到这里drop, 可以在打印中看到FREE
//     // 之后还有很多其他内存被释放
// }

// use std::borrow::Borrow;

// fn main() {
//     let s = "hello world!".to_owned();

//     // 这里必须声明类型, 因为String有多个Borrow<T>实现
//     let r1: &String = s.borrow();

//     let r2: &str = s.borrow();

//     println!("r1: {:p}, r2: {:p}", r1, r2);
// }

// use std::{borrow::Cow, collections::HashMap};
// use url::Url;

// fn main() {
//     let url = Url::parse("http://www.baidu.com/rust?page=1&size=10&header=dir&dr=ald").unwrap();
//     let mut paris = url.query_pairs();
//     // let map = paris.collect::<HashMap<_, _>>();
//     // println!("{:?}", map)

//     let (mut k, v) = paris.next().unwrap();
//     // 此时此刻, 它们都是Borrowed
//     println!("ket: {k}, v: {v}");
//     // 当发生修改时, k变成Owned
//     k.to_mut().push_str("_lala");

//     print_pairs((k, v));

//     print_pairs(paris.next().unwrap());

//     print_pairs(paris.next().unwrap());
// }

// fn print_pairs(pair: (Cow<str>, Cow<str>)) {
//     println!("key: {}, value: {}", show_cow(pair.0), show_cow(pair.1));
// }

// fn show_cow(cow: Cow<str>) -> String {
//     match cow {
//         Cow::Borrowed(v) => format!("Borrowed {v}"),
//         Cow::Owned(v) => format!("Owned {v}"),
//     }
// }

use lazy_static::lazy_static;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// lazy_static宏可以生成复杂的static对象
lazy_static! {
    // 一般情况下Mutex和Arc一起在多线程环境下提供共享内存的使用
    // 如果你把Mutex声明成static, 其中生命周期是静态的, 不需要Arc
    static ref METRICS: Mutex<HashMap<Cow<'static, str>, usize>> = Mutex::new(HashMap::new());
}

fn main() {
    // 用Arc来提供并发环境下的共性所有权(使用引用计数)
    let metrics: Arc<Mutex<HashMap<Cow<'static, str>, usize>>> =
        Arc::new(Mutex::new(HashMap::new()));

    for _ in 0..32 {
        let m = metrics.clone();
        thread::spawn(move || {
            let mut g = m.lock().unwrap();
            // 此时只有拿到MutexGuard的线程可以访问HashMap
            let data = &mut *g;
            // Cow实现了很多数据结构的From trait
            // 所以我们用"hello".into() 生成Cow
            let entry = data.entry("hello".into()).or_insert(0);
            *entry += 1;
            // MutexGuard被Drop, 索贝释放
        });
    }

    thread::sleep(Duration::from_millis(100));

    println!("metrics: {:?}", metrics.lock().unwrap());
}
