// use std::collections::HashMap;
// use std::mem::size_of;

// enum E {
//     A(f64),
//     B(HashMap<String, String>),
//     C(Result<Vec<u8>, String>),
// }

// // 这是一个声明宏, 它会打印各种数据结构本身的带下, 在Option中的大小, 以及在Result中的大小
// macro_rules! show_size {
//     (header) => {
//         println!(
//             "{:<24} {:>4} {} {}",
//             "Type", "T", "Option<T>", "Result<T, io::Error>"
//         );
//         println!("{}", "-".repeat(64));
//     };
//     ($t:ty) => {
//         println!(
//             "{:<24} {:4} {:8} {:12}",
//             stringify!($t),
//             size_of::<$t>(),
//             size_of::<Option<$t>>(),
//             size_of::<Result<$t, std::io::Error>>(),
//         )
//     };
// }

// fn main() {
//     show_size!(header);
//     show_size!(u8);
//     show_size!(f64);
//     show_size!(&u8);
//     show_size!(Box<u8>);
//     show_size!(&[u8]);

//     show_size!(String);
//     show_size!(Vec<u8>);
//     show_size!(HashMap<String, String>);
//     show_size!(E);
// }

// use std::net::SocketAddr;
// fn main() {
//     let addr = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
//     println!("addr: {:?}, port: {:?}", addr.ip(), addr.port());
// }

// use std::io::{BufWriter, Write};
// use std::net::TcpStream;
// #[derive(Debug)]
// struct MyWriter<W> {
//     writer: W,
// }
// impl<W: Write> MyWriter<W> {
//     pub fn new(addr: &str) -> Self {
//         let stream: TcpStream = TcpStream::connect(addr).unwrap();
//         Self {
//             writer: BufWriter::new(stream),
//         }
//     }
//     pub fn write(&mut self, buf: &str) -> std::io::Result<()> {
//         self.writer.write_all(buf.as_bytes())
//     }
// }
// fn main() {
//     let writer: MyWriter<_> = MyWriter::new("127.0.0.1:8080");
//     writer.write("hello world!");
// }

// struct A {
//     len: u8,
//     arr: [u8; 30],
//     size: u16,
// }

// fn main() {
//     println!("{}", std::mem::size_of::<A>());
// }

use std::io::{BufWriter, Write};
use std::net::TcpStream;
#[derive(Debug)]
struct MyWriter<W: Write> {
    writer: W,
}

impl<W: Write> MyWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }
    pub fn write(&mut self, buf: &str) -> std::io::Result<()> {
        self.writer.write_all(buf.as_bytes())
    }
}

fn main() {
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    let mut writer = MyWriter::new(stream);
    writer.write("hello world!");
}
