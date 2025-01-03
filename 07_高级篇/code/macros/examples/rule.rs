#[macro_export]
macro_rules! my_vec {
    () => {
        std::vec::Vec::new()
    };
    // 处理my_vec![1,2,3,4]
    ($($el:expr), *) => ({
        let mut v = std::vec::Vec::new();
        $(v.push($el);)*
        v
    });
    // 处理my_vec![0;10]
    ($el:expr; $n:expr) => {
        std::vec::from_elem($el, $n)
    }
}

fn main() {
    let mut v = my_vec![];
    v.push(1);
    // 调用的时候可以使用[], {}, ()
    let _v = my_vec!(1, 2, 3, 4);
    let _v = my_vec![1, 2, 3, 4];
    let v = my_vec! {1,2,3,4};
    println!("{v:?}");
    //
    let v = my_vec![1; 10];
    println!("{v:?}");
}
