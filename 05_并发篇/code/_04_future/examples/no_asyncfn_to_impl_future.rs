use futures::executor::block_on;
use std::future::Future;

#[tokio::main]
async fn main() {
    let name1 = "Nyh".to_string();
    let name2 = "Lili".to_string();

    say_hello1(&name1).await;
    say_hello2(&name2).await;

    // Future除了可以使用await来执行外, 还可以直接用executor执行
    block_on(say_hello1(&name1));
    block_on(say_hello2(&name2));
}

async fn say_hello1(name: &str) -> usize {
    println!("Hello {name}");
    42
}

fn say_hello2<'fut>(name: &'fut str) -> impl Future<Output = usize> + 'fut {
    async move {
        println!("Hello {name}");
        42
    }
}
