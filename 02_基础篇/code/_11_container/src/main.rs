// fn main() {
//     let arr = [1, 2, 3, 4, 5];
//     let vec = vec![1, 2, 3, 4, 5];
//     let s1 = &arr[..2];
//     let s2 = &vec[..2];
//     println!("s1: {:?}, s2: {:?}", s1, s2);

//     // &[T]和&[T]是否相等取决于长度和内容是否相等
//     assert_eq!(s1, s2);
//     // &[T]可以和Vec<T>/[T;n]比较, 也会看长度和内容
//     assert_eq!(&arr[..], vec);
//     assert_eq!(&vec[..], arr);
// }

// use std::{fmt, ops::Deref};

// fn main() {
//     let v = vec![1, 2, 3, 4];

//     // Vec实现了Deref, &Vec<T>会自动解引用为&[T], 符合接口定义
//     print_slice(&v);

//     // 直接是&[T], 符合接口定义
//     print_slice(&v[..]);

//     // &Vec<T>支持AsRef<T>
//     print_slice1(&v);

//     // &[T]支持AsRef<T>
//     print_slice1(&v[..]);

//     // Vec也支持AsRef<T>
//     print_slice1(v);

//     let arr = [1, 2, 3, 4];
//     // 数组虽然没有实现Deref. 但它的解引用就是&[T]
//     print_slice(&arr);
//     print_slice(&arr[..]);
//     print_slice1(&arr);
//     print_slice1(&arr[..]);
//     print_slice1(arr);
// }

// fn print_slice<T: fmt::Debug>(s: &[T]) {
//     println!("{:?}", s);
// }

// fn print_slice1<T, U>(s: T)
// where
//     T: AsRef<[U]>,
//     U: fmt::Debug,
// {
//     println!("{:?}", s.as_ref());
// }

// fn main() {
//     // 这里Vec<T>在调用iter()时被解引用成&[T], 所以可以访问iter()
//     let result = vec![1, 2, 3, 4]
//         .iter()
//         .map(|v| v * v)
//         .filter(|v| *v < 16)
//         .take(1)
//         .collect::<Vec<_>>();

//     println!("result: {:?}", result)
// }

// use itertools::Itertools;

// fn main() {
//     let err_str = "bad happened";
//     let input = vec![Ok(21), Err(err_str), Ok(7)];
//     let it = input
//         .into_iter()
//         .filter_map_ok(|i| if i > 10 { Some(i * 2) } else { None });
//     println!("{:?}", it.collect::<Vec<_>>());
// }

// use std::ops::Deref;

// fn main() {
//     let s = "sss".to_string();
//     let s1 = s.deref();

// }

// use std::iter::FromIterator;

// fn main() {
//     let arr = ['h', 'e', 'l', 'l', 'o'];
//     let vec = vec!['h', 'e', 'l', 'l', 'o'];
//     let s = String::from("hello");
//     let s1 = &arr[1..3];
//     let s2 = &vec[1..3];
//     // &str本身就是一个特殊的slice
//     let s3 = &s[1..3];
//     println!("s1: {:?}, s2: {:?}, s3: {:?}", s1, s2, s3);

//     // &[char] 和 &[char] 是否相等取决于长度和内容是否相等
//     assert_eq!(s1, s2);
//     // &[char] 和 &str 不能直接对比，我们把 s3 变成 Vec<char>
//     assert_eq!(s2, s3.chars().collect::<Vec<_>>());
//     // &[char] 可以通过迭代器转换成 String，String 和 &str 可以直接对比
//     assert_eq!(String::from_iter(s2), s3);
// }

use std::ops::Deref;

fn main() {
    let mut v1 = vec![1, 2, 3, 4];
    v1.push(5);
    println!("cap should be 8: {}", v1.capacity());

    // 从Vec<T>转换成Box<[T]>, 此时会丢弃多余的capacity
    let b1 = v1.into_boxed_slice();
    let mut b2 = b1.clone();
    let v2 = b1.into_vec();
    println!("cap should be exactly 5: {}", v2.capacity());
    assert!(b2.deref() == v2);

    // Box<[T]>可以更改其内部数据, 但无法push
    b2[0] = 2;
    println!("b2: {:?}", b2);

    // 注意Box<[T]>和Box<[T;n]>并不相同
    let b3 = Box::new([2, 2, 3, 4, 5]);
    println!("b3: {:?}", b3);

    // b2和b3相等, 但b3.deref()无法和v2进行比较
    assert!(b2 == b3);
    let a = b3.deref();
    let b = v2.deref();
    // assert!(b3.deref() == v2);
}
