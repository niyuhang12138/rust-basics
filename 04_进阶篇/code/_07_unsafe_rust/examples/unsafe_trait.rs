// 实现这个trait的开发者要保证实现的是内存安全
unsafe trait Foo {
    fn foo(&self);
}

trait Bar {
    // 调用这个函数的人要保证调用时安全的
    unsafe fn bar(&self);
}

struct Nonsense;

unsafe impl Foo for Nonsense {
    fn foo(&self) {
        println!("Foo");
    }
}

impl Bar for Nonsense {
    unsafe fn bar(&self) {
        println!("Bar");
    }
}

fn main() {
    let nonsense = Nonsense;
    // 调用者无需关心safety
    nonsense.foo();

    // 调用者需要为safety负责
    unsafe {
        nonsense.bar();
    };
}
