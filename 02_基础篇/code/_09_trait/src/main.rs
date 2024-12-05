// use std::fmt;
// use std::io::Write;

// struct BufBuilder {
//     buf: Vec<u8>,
// }

// impl BufBuilder {
//     pub fn new() -> Self {
//         Self {
//             buf: Vec::with_capacity(1024),
//         }
//     }
// }

// impl fmt::Debug for BufBuilder {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", String::from_utf8_lossy(&self.buf))
//     }
// }

// impl Write for BufBuilder {
//     fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
//         // 把buf添加到BufBuilder的尾部
//         self.buf.extend_from_slice(buf);
//         Ok(buf.len())
//     }

//     fn flush(&mut self) -> std::io::Result<()> {
//         // 由于是在内存中操作, 所以不需要flush
//         Ok(())
//     }
// }

// fn main() {
//     let mut buf = BufBuilder::new();
//     buf.write_all(b"Hello World!").unwrap();
// }

// use regex::Regex;

// pub trait Parser {
//     fn parse(s: &str) -> Self;
// }

// impl Parser for u8 {
//     fn parse(s: &str) -> Self {
//         let re = Regex::new(r"^[0-9]+").unwrap();
//         if let Some(captures) = re.captures(s) {
//             // 取第一个match, 将其捕获的digits转换成u8
//             captures
//                 .get(0)
//                 .map_or(0, |s| s.as_str().parse().unwrap_or(0))
//         } else {
//             0
//         }
//     }
// }

// #[test]
// fn parse_should_work() {
//     assert_eq!(u8::parse("123abcd"), 123);
//     assert_eq!(u8::parse("abcd"), 0);
//     assert_eq!(u8::parse("1234abcd"), 1234);
// }

// fn main() {
//     println!("result: {}", u8::parse("255 hello world"));
// }

// use regex::Regex;
// use std::str::FromStr;

// pub trait Parser {
//     type Error;
//     fn parse(s: &str) -> Result<Self, Self::Error>
//     where
//         Self: Sized;
// }

// impl<T> Parser for T
// where
//     T: Default + FromStr,
// {
//     type Error = String;
//     fn parse(s: &str) -> Result<Self, Self::Error>
//     where
//         Self: Sized,
//     {
//         let re = Regex::new(r"^[0-9]+(\.[0-9]+)?").unwrap();
//         if let Some(capture) = re.captures(s) {
//             capture
//                 .get(0)
//                 .map_or(Err("failed to capture".to_string()), |s| {
//                     s.as_str()
//                         .parse()
//                         .map_err(|_err| "failed to parse capture string".to_string())
//                 })
//         } else {
//             Err("failed to parse string".to_string())
//         }
//     }
// }

// #[test]
// fn parse_should_work() {
//     assert_eq!(u8::parse("123abcd"), Ok(123));
// }

// fn main() {
//     println!("result: {:?}", u8::parse("255 hello world"));
// }

// use std::ops::Add;

// #[derive(Debug)]
// struct Complex {
//     real: f64,
//     imagine: f64,
// }

// impl Complex {
//     pub fn new(real: f64, imagine: f64) -> Self {
//         Self { real, imagine }
//     }
// }

// // 对Complex类型的实现
// impl Add for Complex {
//     type Output = Self;

//     // 注意add的第一个参数self, 会移动所有权
//     fn add(self, rhs: Self) -> Self::Output {
//         let real = self.real + rhs.real;
//         let imagine = self.imagine + rhs.imagine;

//         Complex::new(real, imagine)
//     }
// }

// fn main() {
//     let c1 = Complex::new(1.0, 1_f64);
//     let c2 = Complex::new(2 as f64, 3.0);
//     println!("{:?}", c1 + c2);
// }

// use std::ops::Add;

// #[derive(Debug)]
// struct Complex {
//     real: f64,
//     imagine: f64,
// }

// impl Complex {
//     pub fn new(real: f64, imagine: f64) -> Self {
//         Self { real, imagine }
//     }
// }

// // 对Complex类型的实现
// impl Add for &Complex {
//     type Output = Complex;

//     // 注意add的第一个参数self, 会移动所有权
//     fn add(self, rhs: Self) -> Self::Output {
//         let real = self.real + rhs.real;
//         let imagine = self.imagine + rhs.imagine;

//         Complex::new(real, imagine)
//     }
// }

// impl Add<f64> for &Complex {
//     type Output = Complex;

//     fn add(self, rhs: f64) -> Self::Output {
//         let real = self.real + rhs;
//         Complex::new(real, self.imagine)
//     }
// }

// fn main() {
//     let c1 = Complex::new(1.0, 1_f64);
//     let c2 = Complex::new(2 as f64, 3.0);
//     println!("{:?}", &c1 + &c2);
//     println!("{:?}", &c1 + 2.0);
// }

// trait A {
//     fn a() {
//         println!("A::a");
//     }
// }

// trait B {
//     fn b() {
//         println!("B::b");
//     }
// }

// impl<T> A for T where T: B {}

// impl B for i32 {}

// fn main() {
//     let a = 1;
//     <i32 as A>::a();
// }

// struct Cat;
// struct Dog;

// trait Animal {
//     fn name(&self) -> &'static str;
// }

// impl Animal for Cat {
//     fn name(&self) -> &'static str {
//         "Cat"
//     }
// }

// impl Animal for Dog {
//     fn name(&self) -> &'static str {
//         "Dog"
//     }
// }

// fn name(animal: impl Animal) -> &'static str {
//     animal.name()
// }

// fn main() {
//     let cat = Cat;
//     println!("cat: {}", name(cat));
// }

// pub trait Formatter {
//     fn format(&self, input: &mut String) -> bool;
// }

// struct MarkdownFormatter;
// impl Formatter for MarkdownFormatter {
//     fn format(&self, input: &mut String) -> bool {
//         input.push_str("\nformatted with Markdown formatter");
//         true
//     }
// }
// struct RustFormatter;
// impl Formatter for RustFormatter {
//     fn format(&self, input: &mut String) -> bool {
//         input.push_str("\nformatted with Rust formatter");
//         true
//     }
// }
// struct HtmlFormatter;
// impl Formatter for HtmlFormatter {
//     fn format(&self, input: &mut String) -> bool {
//         input.push_str("\nformatted with HTML formatter");
//         true
//     }
// }

// pub fn format(input: &mut String, formatters: Vec<&dyn Formatter>) {
//     for formatter in formatters {
//         formatter.format(input);
//     }
// }

// fn main() {
//     let mut input = "Hello World!".to_string();
//     let formatters: Vec<&dyn Formatter> = vec![&MarkdownFormatter, &RustFormatter, &HtmlFormatter];
//     format(&mut input, formatters);
//     println!("{}", input);
// }

// use std::{fs::File, io::Write};
// fn main() {
//     let mut f = File::create("/tmp/test_write_trait").unwrap();
//     let w: &mut dyn Write = &mut f;
//     w.write_all(b"hello ").unwrap();
//     let w1 = w.by_ref();
//     w1.write_all(b"world").unwrap();
// }

// struct SentenceIter<'a> {
//     s: &'a mut &'a str,
//     delimiter: char,
// }
// impl<'a> SentenceIter<'a> {
//     pub fn new(s: &'a mut &'a str, delimiter: char) -> Self {
//         Self { s, delimiter }
//     }
// }
// impl<'a> Iterator for SentenceIter<'a> {
//     type Item = &'a str; // 想想 Item 应该是什么类型？
//     fn next(&mut self) -> Option<Self::Item> {
//         // 如何实现 next 方法让下面的测试通过？
//         if self.s.is_empty() {
//             return None;
//         }

//         match self.s.find(self.delimiter) {
//             Some(pos) => {
//                 let len = self.delimiter.len_utf8();
//                 let s = &self.s[..pos + len];
//                 let suffix = &self.s[pos + len..];
//                 *self.s = suffix;
//                 Some(s.trim())
//             }
//             None => {
//                 let s = (*self.s).trim();
//                 *self.s = "";
//                 if s.len() == 0 {
//                     return None;
//                 } else {
//                     Some(s)
//                 }
//             }
//         }
//     }
// }
// #[test]
// fn it_works() {
//     let mut s = "This is the 1st sentence. This is the 2nd sentence.";
//     let mut iter = SentenceIter::new(&mut s, '.');
//     assert_eq!(iter.next(), Some("This is the 1st sentence."));
//     assert_eq!(iter.next(), Some("This is the 2nd sentence."));
//     assert_eq!(iter.next(), None);
// }
// fn main() {
//     let mut s = "a。 b。 c";
//     let sentences: Vec<_> = SentenceIter::new(&mut s, '。').collect();
//     println!("sentences: {:?}", sentences);
// }

// #[derive(Clone, Debug, Copy)]
// struct Developer {
//     name: String,
//     age: u8,
//     lang: Language,
// }

// #[allow(dead_code)]
// #[derive(Clone, Debug, Copy)]
// enum Language {
//     Rust,
//     TypeScript,
//     Elixir,
//     Haskell,
// }

// fn main() {
//     let dev = Developer {
//         name: "Tyr".to_string(),
//         age: 19,
//         lang: Language::Rust,
//     };

//     let dev1 = dev.clone();

//     println!("dev: {:?}", dev);
//     println!("dev1: {:?}", dev1);

//     println!("dev: {:?}, addr of dev name: {:p}", dev, dev.name.as_str());
//     println!(
//         "dev1: {:?}, addr of dev1 name: {:p}",
//         dev1,
//         dev1.name.as_str()
//     );
// }

// use std::{fmt, slice, vec};

// // #[derive(Clone, Copy)]
// #[derive(Clone)]
// struct RawBuffer {
//     // 裸指针用 *const / *mut 来表述, 这里和引用的&不同
//     ptr: *mut u8,
//     len: usize,
// }

// impl From<Vec<u8>> for RawBuffer {
//     fn from(value: Vec<u8>) -> Self {
//         let slice = value.into_boxed_slice();
//         Self {
//             len: slice.len(),
//             // into_raw之后, Box就不管这块内存的释放了, RawBuffer需要处理释放
//             ptr: Box::into_raw(slice) as *mut u8,
//         }
//     }
// }

// // 如果RawBuffer实现了Drop trait, 就可以在所有者提出时释放内存
// // 然后Drop trait会跟Copy trait冲突, 要不实现Copy trait要不实现Drop trait
// // 如果不实现Drop trait, 那么就会导致内存泄露, 但它不会对正确性有任何破坏
// // 比如不会出现use after free这样的问题
// impl Drop for RawBuffer {
//     #[inline]
//     fn drop(&mut self) {
//         let data = unsafe {
//             let _ = Box::from_raw(slice::from_raw_parts_mut(self.ptr, self.len));
//         };
//         drop(data);
//     }
// }

// impl fmt::Debug for RawBuffer {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let data = self.as_ref();
//         write!(f, "{:p}: {:?}", self.ptr, data)
//     }
// }
// impl AsRef<[u8]> for RawBuffer {
//     fn as_ref(&self) -> &[u8] {
//         unsafe { slice::from_raw_parts(self.ptr, self.len) }
//     }
// }

// fn main() {
//     let data = vec![1, 2, 3, 4];
//     let buf: RawBuffer = data.into();
//     use_buffer(&buf);
//     println!("buf: {:?}", buf);
// }

// fn use_buffer(buf: &RawBuffer) {
//     println!("buf to die: {:?}", buf);
//     // drop(buf);
// }

// use std::rc::Rc;
// fn rc_is_not_send_and_sync() {
//     let a = Rc::new(1);
//     let b = a.clone();
//     let c = a.clone();
//     std::thread::spawn(move || {
//         println!("c = {:?}", c);
//     })
// }

// fn main() {}

// use std::cell::RefCell;

// fn refcell_is_send() {
//     let a = RefCell::new(1);
//     std::thread::spawn(move || println!("a: {:?}", a));
// }

// fn main() {
//     refcell_is_send();
// }

// use std::{cell::RefCell, sync::Arc};

// // RefCell现在有多个Arc持有它, 虽然Arc是Send/Sync, 但RefCell不是Sync
// fn refcell_is_not_sync() {
//     let a = Arc::new(RefCell::new(1));
//     let b = a.clone();
//     let c  = a.clone();
//     std::thread::spawn(move || {
//         println!("c = {:?}", c);
//     });
// }

// use std::sync::{Arc, Mutex};

// // RefCell现在有多个Arc持有它, 虽然Arc是Send/Sync, 但RefCell不是Sync
// fn arc_mutex_is_send_sync() {
//     let a = Arc::new(Mutex::new(1));
//     let b = a.clone();
//     let c = a.clone();
//     let handle = std::thread::spawn(move || {
//         let mut g = c.lock().unwrap();
//         *g += 1;
//     });

//     {
//         let mut g = b.lock().unwrap();
//         *g += 1;
//     }

//     handle.join().unwrap();
//     println!("a = {:?}", a);
// }

// fn main() {
//     arc_mutex_is_send_sync();
// }

// use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// fn print(v: impl Into<IpAddr>) {
//     println!("{:?}", v.into());
// }

// fn main() {
//     let v4 = "2.2.2.2".parse::<Ipv4Addr>().unwrap();
//     let v6 = "::1".parse::<Ipv6Addr>().unwrap();
//     print([1, 1, 1, 1]);
//     print(v4);
//     print(v6);
// }

// #[derive(Debug)]
// struct Person {
//     name: String,
//     age: u8,
//     address: String,
// }

// impl From<String> for Person {
//     fn from(value: String) -> Self {
//         let sp = value.split(',');
//         println!("sp: {:?}", sp);
//         Self {
//             name: "zs".to_string(),
//             age: 19,
//             address: "山东".to_string(),
//         }
//     }
// }

// fn main() {
//     let p = Person {
//         name: "zs".to_string(),
//         age: 19,
//         address: "山东".to_string(),
//     };

//     println!("Person: {:?}", p);
//     let p1 = Person::from("zs,19,山东".to_string());
//     println!("p1: {:?}", p1);
//     let p2: Person = "saciasco".to_string().into();
//     println!("p2: {:?}", p2);
// }

// #[allow(dead_code)]
// enum Language {
//     Rust,
//     TypeScript,
//     Elixir,
//     Haskell,
// }

// impl AsRef<str> for Language {
//     fn as_ref(&self) -> &str {
//         match &self {
//             Language::Rust => "Rust",
//             Language::TypeScript => "TypeScript",
//             Language::Elixir => "Elixir",
//             Language::Haskell => "Haskell",
//         }
//     }
// }

// fn print_ref(v: impl AsRef<str>) {
//     println!("{}", v.as_ref());
// }

// fn main() {
//     let lang = Language::Rust;

//     // &str实现了AsRef<str>
//     print_ref("Hello World!");

//     // String实现了AsRef<str>
//     print_ref("Hello World!".to_string());

//     // 我们自己定义的枚举也实现了AsRef<str>
//     print_ref(lang);
// }

// use std::ops::{Deref, DerefMut};

// #[derive(Debug)]
// struct Buffer<T>(Vec<T>);

// impl<T> Buffer<T> {
//     pub fn new(v: impl Into<Vec<T>>) -> Self {
//         Self(v.into())
//     }
// }

// impl<T> Deref for Buffer<T> {
//     type Target = [T];

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl<T> DerefMut for Buffer<T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

// fn main() {
//     let mut buf = Buffer::new([1, 2, 3, 4]);
//     // 因为实现了Deref和DerefMut, 这里buf可以直接访问Vec<T>的方法
//     // 下面这句相当于: (&mut buf).deref_mut().sort() / (&mut buf.0).sort()
//     buf.sort();
//     println!("buf: {:?}", buf)
// }

// use std::fmt;

// // struct可以derive Default, 但我们需要所有的字段都实现了Default
// #[derive(Debug, Clone, Default)]
// struct Developer {
//     name: String,
//     age: u8,
//     lang: Language,
// }

// // enum不能derive Default
// #[allow(dead_code)]
// #[derive(Clone, Debug)]
// enum Language {
//     Rust,
//     TypeScript,
//     Elixir,
//     Haskell,
// }

// // 手工实现Default
// impl Default for Language {
//     fn default() -> Self {
//         Language::Rust
//     }
// }

// impl Developer {
//     pub fn new(name: &str) -> Self {
//         // 用..Default::default() 为剩余字段使用缺省值
//         Self {
//             name: name.to_owned(),
//             ..Default::default()
//         }
//     }
// }

// impl fmt::Display for Developer {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "{}({} years old); {:?} developer",
//             self.name, self.age, self.lang
//         )
//     }
// }

// fn main() {
//     // 使用T::default()
//     let dev1 = Developer::default();
//     // 使用Default::default(), 但此时类型无法通过上下文推断, 需要提供类型
//     let dev2: Developer = Default::default();
//     // 使用 T::new
//     let dev3 = Developer::new("Tyr");
//     println!("dev1: {}\\ndev2: {}\\ndev3: {:?}", dev1, dev2, dev3);
// }

// use std::ops::Deref;

// fn main() {
//     use std::sync::{Arc, Mutex};
//     let shared = Arc::new(Mutex::new(1));
//     let mut g = shared.lock().unwrap();
//     *g += 1;
// }

use std::{
    collections::LinkedList,
    ops::{Deref, DerefMut, Index},
};
struct List<T>(LinkedList<T>);

impl<T> Deref for List<T> {
    type Target = LinkedList<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for List<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T> Default for List<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<T> Index<isize> for List<T> {
    type Output = T;
    fn index(&self, index: isize) -> &Self::Output {
        let len = self.len() as isize;
        if len == 0 {
            panic!("Convert index into an empty list");
        }

        let mut idx = if index < 0 { len + index } else { index };

        if idx < 0 || idx >= len {
            // panic!("index out of bounds: the len is {len} but the index is {index}");
            idx = 0;
        }

        self.iter()
            .nth(idx as usize)
            .expect("Indexing into a LinkedList failed")
    }
}
#[test]
fn it_works() {
    let mut list: List<u32> = List::default();
    for i in 0..16 {
        list.push_back(i);
    }
    assert_eq!(list[0], 0);
    assert_eq!(list[5], 5);
    assert_eq!(list[15], 15);
    assert_eq!(list[16], 0);
    assert_eq!(list[-1], 15);
    assert_eq!(list[128], 0);
    assert_eq!(list[-128], 0);
}

fn main() {
    Box
}