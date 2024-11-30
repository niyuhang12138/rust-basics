/* 函数作为参数 */
// fn apply(value: i32, f: fn(i32) -> i32) -> i32 {
//     f(value)
// }

// fn square(value: i32) -> i32 {
//     value * value
// }

// fn cube(value: i32) -> i32 {
//     value * value * value
// }

// fn main() {
//     println!("apply square: {}", apply(2, square));
//     println!("apply cube: {}", apply(2, cube));
// }

/* 聊天服务数据结构 */

// #[derive(Debug)]
// enum Gender {
//     Unspecified = 0,
//     Female = 1,
//     Male = 2,
// }

// #[derive(Debug, Copy, Clone)]
// struct UserId(u64);

// #[derive(Debug, Copy, Clone)]
// struct TopicId(u64);

// #[derive(Debug)]
// struct User {
//     id: UserId,
//     name: String,
//     gender: Gender,
// }

// #[derive(Debug)]
// struct Topic {
//     id: TopicId,
//     name: String,
//     owner: UserId,
// }

// // 定义聊天室可能发生的事件
// #[derive(Debug)]
// enum Event {
//     Join((UserId, TopicId)),
//     Leave((UserId, TopicId)),
//     Message((UserId, TopicId, String)),
// }

// fn main() {
//     let alice = User {
//         id: UserId(1),
//         name: "Alice".into(),
//         gender: Gender::Female,
//     };
//     let bob = User {
//         id: UserId(2),
//         name: "bob".into(),
//         gender: Gender::Male,
//     };

//     let topic = Topic {
//         id: TopicId(1),
//         name: "rust".into(),
//         owner: UserId(1),
//     };

//     let event1 = Event::Join((alice.id, topic.id));
//     let event2 = Event::Join((bob.id, topic.id));
//     let event3 = Event::Message((alice.id, topic.id, "Hello World!".into()));

//     println!(
//         "event1: {:?}, event2: {:?}, event3: {:?}",
//         event1, event2, event3
//     )
// }

// #[derive(Debug, Copy, Clone)]
// struct Id(i32);

// fn main() {
//     let id_1 = Id(1);
//     let id_2 = id_1;
//     println!("id_1: {:?}, id_2: {:?}", id_1, id_2);
// }

/* 斐波那契数列 */

// fn fib_loop(n: i32) -> i32 {
//     if n <= 1 {
//         return n;
//     }

//     let mut a = 0;
//     let mut b = 1;
//     let mut i = 2;

//     loop {
//         let temp: i32 = a + b;
//         a = b;
//         b = temp;
//         i += 1;

//         if i >= n {
//             return temp;
//         }
//     }
// }

// fn fib(n: i128) -> i128 {
//     if n <= 1 {
//         return n;
//     }

//     let mut a = 0;
//     let mut b = 1;
//     let mut temp = 0;

//     for _i in 2..n {
//         temp = a + b;
//         a = b;
//         b = temp;
//     }

//     return temp;
// }

// fn main() {
//     println!("n = 10: {}", fib_loop(10));
//     println!("n = 10: {}", fib(10));
// }

// fn process_event(event: &Event) {
//     match event {
//         Event::Join((uid, _tid)) => ..,
//         Event::Leave((uid, _tid)) => ..,
//         Event::Message((_, _, msg)) => ..,
//     }
// }
