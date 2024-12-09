// use std::collections::HashMap;

// fn main() {
//     // 长度为0
//     let c1 = || println!("hello world!");
//     // 和参数无关, 长度也为0
//     let c2 = |i: i32| println!("hello: {i}");
//     let name = String::from("tyr");
//     let name1 = name.clone();
//     let mut table = HashMap::new();
//     table.insert("hello", "world");
//     // 如果捕获一个引用, 长度为8
//     let c3 = || println!("hello: {name}");
//     // 捕获移动的数据name1(长度24) + table(长度48), closure长度72
//     let c4 = move || println!("hello: {}, {:?}", name1, table);
//     let name2 = name.clone();
//     // 和局部变量无关, 捕获一个String name2, closure长度为24
//     let c5 = move || {
//         let x = 1;
//         let name3 = String::from("lindsey");
//         println!("hello: {}, {:?}, {:?}", x, name2, name3);
//     };

//     println!(
//         "c1: {}, c2: {}, c3: {}, c4: {}, c5:{}, main: {}",
//         size_of_val(&c1),
//         size_of_val(&c2),
//         size_of_val(&c3),
//         size_of_val(&c4),
//         size_of_val(&c5),
//         size_of_val(&main)
//     );
// }

// fn main() {
//     let name = String::from("Tyr");
//     let c = move |greeting: String| (greeting, name);
//     let result = c("hello".to_string());
//     println!("result: {:?}", result);
//     // 无法再次调用
//     // let result = c("hi".to_string());
// }

// fn main() {
//     let name = String::from("Tyr");

//     // 这个闭包会clone内部额数据返回, 所以它不是FnOnce
//     let c = move |greeting: String| (greeting, name.clone());

//     // 所以c1可以被调用多次
//     println!("c1 call once: {:?}", c("qiao".into()));
//     println!("c1 call twice: {:?}", c("bonjour".into()));

//     // 然后一旦被当成FnOnce被调用, 就无法再次调用
//     println!("result: {:?}", call_once("h1".into(), c));

//     // 无法再次调用
//     // let result = c("he".into());

//     // fn也可以被当成fnOnce调用, 只要结构一致接可以
//     println!("result: {:?}", call_once("hola".into(), not_closure))
// }

// fn call_once(arg: String, c: impl FnOnce(String) -> (String, String)) -> (String, String) {
//     c(arg)
// }

// fn not_closure(arg: String) -> (String, String) {
//     (arg, "Rosie".into())
// }

// fn main() {
//     let mut name = String::from("hello");
//     let mut name1 = String::from("hello");

//     let mut c = || {
//         name.push_str("!");
//         println!("c: {:?}", name);
//     };

//     let mut c1 = move || {
//         name1.push_str("!");
//         println!("c1: {:?}", name1);
//     };

//     c();
//     c1();

//     call_mut(&mut c);
//     call_once(&mut c1);

//     call_once(c);
//     call_once(c1);
// }

// // 在作为参数时, FnMut需要显示的使用mut或者&mut
// fn call_mut(c: &mut impl FnMut()) {
//     c();
// }

// fn call_once(c: impl FnOnce()) {
//     c();
// }

fn main() {
    let v = vec![0_u8; 1024];
    let v1 = vec![0_u8; 1023];

    // Fn, 不移动所有权
    let mut c = |x: u64| v.len() as u64 * x;
    // Fn 移动所有权
    let mut c1 = move |x: u64| v1.len() as u64 * x;

    println!("direct call: {}", c(2));
    println!("direct call: {}", c1(2));

    println!("call: {}", call(3, &c));
    println!("call: {}", call(3, &c1));

    println!("call_mut: {}", call_mut(4, &mut c));
    println!("call_mut: {}", call_mut(4, &mut c1));

    println!("call_once: {}", call_once(5, c));
    println!("call_once: {}", call_once(5, c1));
}

fn call(arg: u64, c: &impl Fn(u64) -> u64) -> u64 {
    c(arg)
}

fn call_mut(arg: u64, c: &mut impl FnMut(u64) -> u64) -> u64 {
    c(arg)
}

fn call_once(arg: u64, c: impl FnOnce(u64) -> u64) -> u64 {
    c(arg)
}
