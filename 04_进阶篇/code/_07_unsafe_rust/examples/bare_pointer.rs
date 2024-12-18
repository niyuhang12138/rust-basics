use std::ops::Add;

fn main() {
    let mut age = 28;

    // 不可变指针
    let r1 = &age as *const i32;

    // 可变指针
    let r2 = &mut age as *mut i32;

    // 使用裸指针, 可以绕过immutable / mutable borrow rule

    // 然而, 对指针解引用需要使用unsafe
    unsafe {
        println!("r1: {}, r2: {}", *r1, *r2);
    }
}

// fn immutable_mutable_cant_coexist() {
//     let mut age = 19;

//     let r1 = &age;
//     let r2 = &mut age;

//     println!("r1: {}, r2: {}", r1, r2);
// }
