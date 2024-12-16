use std::io::BufReader;

fn main() {}

trait A {}

struct B;

impl A for B {}

fn return_impl_trait() -> impl A {
    B
}
