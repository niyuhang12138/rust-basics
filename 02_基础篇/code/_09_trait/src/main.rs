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
