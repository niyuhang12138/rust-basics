// use std::pin::Pin;

// fn main() {
//     let data = 5;
//     let pinned_data = Pin::new(&data);
//     foo(pinned_data);
//     println!("pinned_data: {pinned_data}");
// }

// fn foo(a: Pin<&i32>) {
//     println!("{a}");
// }

// #[derive(Debug)]
// struct SelfReference {
//     name: String,
//     name_ptr: *const String,
// }

// impl SelfReference {
//     pub fn new(name: impl Into<String>) -> Self {
//         SelfReference {
//             name: name.into(),
//             name_ptr: std::ptr::null(),
//         }
//     }
//     pub fn init(&mut self) {
//         self.name_ptr = &self.name as *const String;
//     }
//     pub fn print_name(&self) {
//         println!(
//             "struct {:p}: (name: {:p} name_ptr: {:p}), name: {}, name_ref: {}",
//             self,
//             &self.name,
//             self.name_ptr,
//             self.name,
//             // 在使用 ptr 是需要 unsafe
//             // SAFETY: 这里 name_ptr 潜在不安全，会指向旧的位置
//             unsafe { &*self.name_ptr },
//         );
//     }
// }
// fn main() {
//     let data = move_creates_issue();
//     println!("data: {:?}", data);
//     // 如果把下面这句注释掉，程序运行会直接 segment error
//     // data.print_name();
//     print!("\\n");
//     mem_swap_creates_issue();
// }

// fn move_creates_issue() -> SelfReference {
//     let mut data = SelfReference::new("Tyr");
//     data.init();
//     // 不 move，一切正常
//     data.print_name();
//     let data = move_it(data);
//     // move 之后，name_ref 指向的位置是已经失效的地址
//     // 只不过现在 move 前的地址还没被回收挪作它用
//     data.print_name();
//     data
// }
// fn mem_swap_creates_issue() {
//     let mut data1 = SelfReference::new("Tyr");
//     data1.init();
//     let mut data2 = SelfReference::new("Lindsey");
//     data2.init();
//     data1.print_name();
//     data2.print_name();
//     std::mem::swap(&mut data1, &mut data2);
//     data1.print_name();
//     data2.print_name();
// }
// fn move_it(data: SelfReference) -> SelfReference {
//     data
// }

use std::{marker::PhantomPinned, pin::Pin};
#[derive(Debug)]
struct SelfReference {
    name: String,
    // 在初始化后指向 name
    name_ptr: *const String,
    // PhantomPinned 占位符
    _marker: PhantomPinned,
}
impl SelfReference {
    pub fn new(name: impl Into<String>) -> Self {
        SelfReference {
            name: name.into(),
            name_ptr: std::ptr::null(),
            _marker: PhantomPinned,
        }
    }
    pub fn init(self: Pin<&mut Self>) {
        let name_ptr = &self.name as *const String;
        // SAFETY: 这里并不会把任何数据从 &mut SelfReference 中移走
        let this = unsafe { self.get_unchecked_mut() };
        this.name_ptr = name_ptr;
    }
    pub fn print_name(self: Pin<&Self>) {
        println!(
            "struct {:p}: (name: {:p} name_ptr: {:p}), name: {}, name_ref: {}",
            self,
            &self.name,
            self.name_ptr,
            self.name,
            // 在使用 ptr 是需要 unsafe
            // SAFETY: 因为数据不会移动，所以这里 name_ptr 是安全的
            unsafe { &*self.name_ptr },
        );
    }
}
fn main() {
    move_creates_issue();
}
fn move_creates_issue() {
    let mut data = SelfReference::new("Tyr");
    let mut data = unsafe { Pin::new_unchecked(&mut data) };
    SelfReference::init(data.as_mut());
    // 不 move，一切正常
    data.as_ref().print_name();
    // 现在只能拿到 pinned 后的数据，所以 move 不了之前
    move_pinned(data.as_mut());
    println!("{:?} ({:p})", data, &data);
    // 你无法拿回 Pin 之前的 SelfReference 结构，所以调用不了 move_it
    // move_it(data);
}
fn move_pinned(data: Pin<&mut SelfReference>) {
    println!("{:?} ({:p})", data, &data);
}
#[allow(dead_code)]
fn move_it(data: SelfReference) {
    println!("{:?} ({:p})", data, &data);
}
