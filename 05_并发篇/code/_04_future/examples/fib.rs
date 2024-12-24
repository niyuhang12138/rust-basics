use std::task::Poll;

use futures::prelude::*;
use futures::stream::poll_fn;

#[tokio::main]
async fn main() {
    consume(fib().take(10)).await;
    consume(fib1(10)).await;
    // unfold产生的Unfold stream没有实现Unpin
    // 所以我们将其Pin<Box<T>>一下, 使其满足consume的接口
    consume(fib2(10).boxed()).await;
}

async fn consume(mut st: impl Stream<Item = i32> + Unpin) {
    while let Some(v) = st.next().await {
        println!("{v}");
    }
    println!("\\n");
}

// 使用repeat_with创建stream, 无法控制何时结束
fn fib() -> impl Stream<Item = i32> {
    let mut a = 1;
    let mut b = 1;
    stream::repeat_with(move || {
        let c = a + b;
        a = b;
        b = c;
        b
    })
}

// 使用poll_fn创建stream, 可以通过返回Poll::Ready(None)来结束
fn fib1(mut n: usize) -> impl Stream<Item = i32> {
    let mut a = 1;
    let mut b = 1;
    poll_fn(move |_cx| -> Poll<Option<i32>> {
        if n == 0 {
            return Poll::Ready(None);
        }

        n -= 1;
        let c = a + b;
        a = b;
        b = c;
        Poll::Ready(Some(b))
    })
}

fn fib2(n: usize) -> impl Stream<Item = i32> {
    stream::unfold((n, (1, 1)), |(mut n, (a, b))| async move {
        if n == 0 {
            None
        } else {
            n -= 1;
            let c = a + b;
            // c作为poll_next的返回值, (n, (a, b))作为state
            Some((c, (n, (b, c))))
        }
    })
}
