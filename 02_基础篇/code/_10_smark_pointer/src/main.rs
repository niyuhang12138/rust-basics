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

// use lazy_static::lazy_static;
// use std::borrow::Cow;
// use std::collections::HashMap;
// use std::sync::{Arc, Mutex};
// use std::thread;
// use std::time::Duration;

// // lazy_static宏可以生成复杂的static对象
// lazy_static! {
//     // 一般情况下Mutex和Arc一起在多线程环境下提供共享内存的使用
//     // 如果你把Mutex声明成static, 其中生命周期是静态的, 不需要Arc
//     static ref METRICS: Mutex<HashMap<Cow<'static, str>, usize>> = Mutex::new(HashMap::new());
// }

// fn main() {
//     // 用Arc来提供并发环境下的共性所有权(使用引用计数)
//     let metrics: Arc<Mutex<HashMap<Cow<'static, str>, usize>>> =
//         Arc::new(Mutex::new(HashMap::new()));

//     for _ in 0..32 {
//         let m = metrics.clone();
//         thread::spawn(move || {
//             let mut g = m.lock().unwrap();
//             // 此时只有拿到MutexGuard的线程可以访问HashMap
//             let data = &mut *g;
//             // Cow实现了很多数据结构的From trait
//             // 所以我们用"hello".into() 生成Cow
//             let entry = data.entry("hello".into()).or_insert(0);
//             *entry += 1;
//             // MutexGuard被Drop, 索贝释放
//         });
//     }

//     thread::sleep(Duration::from_millis(100));

//     println!("metrics: {:?}", metrics.lock().unwrap());
// }

// fn main() {
//     let mut b = Box::new(1);
//     println!("b: {b}");
//     *b += 1;
// }

// use std::borrow::{Borrow, Cow};

// fn main() {
//     let s = "hello world!";
//     let mut s1 = Cow::from(s);
//     // s1.to_mut().push_str("string");
//     let s2 = s1.to_owned();
//     println!("s2: {:?}", s2);
//     match s1 {
//         Cow::Borrowed(b) => println!("Borrowed b: {b}"),
//         Cow::Owned(b) => println!("Owned b: {b}"),
//     }
//     match s2 {
//         Cow::Borrowed(b) => println!("Borrowed b: {b}"),
//         Cow::Owned(b) => println!("Owned b: {b}"),
//     }
// }

// use std::sync::Mutex;

// fn main() {
//     let a = Mutex::new(1);
//     let l = a.lock().unwrap();
// }

// use core::str;
// use std::{fmt, ops::Deref};

// const MINI_STRING_MAX_LEN: usize = 30;

// // MyString里, String有三个word, 共24字节, 所以它以8字节对齐
// // 所以enum的tag + padding最少8字节, 整个结构栈32字节
// // MiniString可以最多有30字节(在加上1字节长度和1字节tag), 就是32字节

// struct MiniString {
//     len: u8,
//     data: [u8; MINI_STRING_MAX_LEN],
// }

// impl MiniString {
//     // 这里的new接口不暴露出去, 保证传入的v的字节长度小于等于30
//     fn new(v: impl AsRef<str>) -> Self {
//         let bytes = v.as_ref().as_bytes();
//         // 我们在拷贝内容时一定要使用字符串的字节长度
//         let len: usize = bytes.len();
//         let mut data: [u8; 30] = [0_u8; MINI_STRING_MAX_LEN];
//         data[..len].copy_from_slice(bytes);
//         Self {
//             len: len as u8,
//             data,
//         }
//     }
// }

// impl Deref for MiniString {
//     type Target = str;

//     fn deref(&self) -> &Self::Target {
//         // 由于生成MiniString的接口是隐藏的, 它只能来自字符串, 所以下面折行是安全的
//         str::from_utf8(&self.data[..self.len as usize]).unwrap()
//     }
// }

// impl fmt::Debug for MiniString {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.deref())
//     }
// }

// #[derive(Debug)]
// enum MyString {
//     Inline(MiniString),
//     Standard(String),
// }

// impl MyString {
//     pub fn push_str(&mut self, s: &str) {
//         match *self {
//             MyString::Inline(ref mut v) => {
//                 let len = v.len as usize;
//                 let len1 = s.len();
//                 if len + len1 > MINI_STRING_MAX_LEN {
//                     let mut owned = v.deref().to_string();
//                     owned.push_str(s);
//                     *self = MyString::Standard(owned);
//                 } else {
//                     let total = len + len1;
//                     v.data[len..len + len1].copy_from_slice(s.as_bytes());
//                     v.len = total as u8
//                 }
//             }
//             MyString::Standard(ref mut v) => v.push_str(s),
//         }
//     }
// }

// // 实现Deref接口对两种不同场景同一得到&str
// impl Deref for MyString {
//     type Target = str;

//     fn deref(&self) -> &Self::Target {
//         match *self {
//             MyString::Inline(ref v) => v.deref(),
//             MyString::Standard(ref v) => v.deref(),
//         }
//     }
// }

// // impl From<&str> for MyString {
// //     fn from(value: &str) -> Self {
// //         match value.len() > MINI_STRING_MAX_LEN {
// //             true => Self::Standard(value.to_owned()),
// //             _ => Self::Inline(MiniString::new(value)),
// //         }
// //     }
// // }

// // impl From<String> for MyString {
// //     fn from(value: String) -> Self {
// //         println!("String: len - {}", value.len());
// //         match value.len() > MINI_STRING_MAX_LEN {
// //             true => Self::Standard(value),
// //             _ => Self::Inline(MiniString::new(value)),
// //         }
// //     }
// // }

// impl<T> From<T> for MyString
// where
//     T: AsRef<str> + Into<String>,
// {
//     fn from(value: T) -> Self {
//         match value.as_ref().len() > MINI_STRING_MAX_LEN {
//             true => Self::Standard(value.into()),
//             _ => Self::Inline(MiniString::new(value)),
//         }
//     }
// }

// impl fmt::Display for MyString {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.deref())
//     }
// }

// fn main() {
//     let len1 = std::mem::size_of::<MyString>();
//     let len2 = std::mem::size_of::<MiniString>();
//     println!("Len: MyString {len1}, MiniString {len2}");

//     let s1: MyString = "hello world".into();
//     let s2: MyString = "这是一个超过了三十个字节的很长很长很长的字符串".into();

//     println!("s1: {:?}, s2: {:?}", s1, s2);

//     println!(
//         "s1: {}({} bytes, {} chars), s2: {}({} bytes, {} chars)",
//         s1,
//         s1.len(),
//         s1.chars().count(),
//         s2,
//         s2.len(),
//         s2.chars().count()
//     );

//     assert!(s1.ends_with("world"));
//     assert!(s2.starts_with("这"));

//     let mut ss1: MyString = "abcd".to_string().into();
//     let ss2: MyString = "这是一个超过了三十个字节的很长很长很长的字符串"
//         .to_string()
//         .into();
//     println!("ss1: {:?}, ss2: {:?}", ss1, ss2);

//     ss1.push_str("efg");
//     println!("ss1: {:?}, ss2: {:?}", ss1, ss2);
// }

use std::borrow::Cow;

fn main() {
    let s1 = std::mem::size_of::<Cow<u8>>();
    let s2 = std::mem::size_of::<Cow<str>>();
    println!("s1: {s1}, s2: {s2}")
}
