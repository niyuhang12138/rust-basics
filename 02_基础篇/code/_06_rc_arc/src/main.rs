// use std::rc::Rc;

// fn main() {
//     let a = Rc::new(1);
//     let b = a.clone();
//     let c = a.clone();
// }

// /// 有向无环图

// use std::rc::{self, Rc};

// #[derive(Debug)]
// struct Node {
//     id: usize,
//     downstream: Option<Rc<Node>>,
// }

// impl Node {
//     pub fn new(id: usize) -> Self {
//         Self {
//             id,
//             downstream: None,
//         }
//     }

//     pub fn update_downstream(&mut self, downstream: Rc<Node>) {
//         self.downstream = Some(downstream);
//     }

//     pub fn get_downstream(&self) -> Option<Rc<Node>> {
//         self.downstream.as_ref().map(|v| v.clone())
//     }
// }

// fn main() {
//     let mut node1 = Node::new(1);
//     let mut node2 = Node::new(2);
//     let mut node3 = Node::new(3);
//     let node4 = Node::new(4);
//     node3.update_downstream(Rc::new(node4));
//     node1.update_downstream(Rc::new(node3));
//     node2.update_downstream(node1.get_downstream().unwrap());
//     println!("node1: {:#?}, node2: {:#?}", node1, node2);

//     let node5 = Node::new(5);
//     let node3 = node1.get_downstream().unwrap();
//     node3.update_downstream(Rc::new(node5));
//     println!("node1: {:#?}, node2: {:#?}", node1, node2);
// }

// use std::cell::RefCell;

// fn main() {
//     let data = RefCell::new(1);
//     {
//         // 获得RefCell内部数据的可变借用
//         let mut v = data.borrow_mut();
//         *v += 1;
//     }
//     println!("data: {:?}", data.borrow());
// }

// use std::cell::RefCell;

// fn main() {
//     let data = RefCell::new(1);
//     // 获得RefCell内部数据的可变借用
//     let mut v = data.borrow_mut();
//     *v += 1;
//     println!("data: {:?}", data.borrow());
// }

// #[derive(Debug)]
// struct Person {
//     name: String,
//     age: u8,
// }

// fn main() {
//     let mut a = Person {
//         name: "zs".to_string(),
//         age: 10,
//     };
//     let b = &mut a;
//     b.age += 1;
//     let c = &a;
//     println!("a: {:?}", c)
// }

// /// 有向无环图
// use std::{
//     cell::RefCell,
//     rc::{self, Rc},
// };

// #[derive(Debug)]
// struct Node {
//     id: usize,
//     downstream: Option<Rc<RefCell<Node>>>,
// }

// impl Node {
//     pub fn new(id: usize) -> Self {
//         Self {
//             id,
//             downstream: None,
//         }
//     }

//     pub fn update_downstream(&mut self, downstream: Rc<RefCell<Node>>) {
//         self.downstream = Some(downstream);
//     }

//     pub fn get_downstream(&self) -> Option<Rc<RefCell<Node>>> {
//         self.downstream.as_ref().map(|v| v.clone())
//     }
// }

// fn main() {
//     let mut node1 = Node::new(1);
//     let mut node2 = Node::new(2);
//     let mut node3 = Node::new(3);
//     let node4 = Node::new(4);
//     node3.update_downstream(Rc::new(RefCell::new(node4)));
//     node1.update_downstream(Rc::new(RefCell::new(node3)));
//     node2.update_downstream(node1.get_downstream().unwrap());
//     println!("node1: {:#?}, node2: {:#?}", node1, node2);

//     let node5 = Node::new(5);
//     let node3 = node1.get_downstream().unwrap();
//     // node3.update_downstream(Rc::new(RefCell::new(node5)));
//     node3.borrow_mut().downstream = Some(Rc::new(RefCell::new(node5)));
//     println!("node1: {:#?}, node2: {:#?}", node1, node2);
// }

// fn main() {
//     let arr = vec![1];
//     std::thread::spawn(move || {
//         println!("{:?}", arr);
//     });
// }

use std::{rc::Rc, sync::Arc};

fn main() {
    let s = Arc::new("hello");
    let s1 = s.clone();
    std::thread::spawn(move || {
        println!("{}", s1);
    });

    println!("s: {s}");
}
