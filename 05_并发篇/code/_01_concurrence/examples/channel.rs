// mpsc

use std::{
    sync::{mpsc, Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

// use actix::{clock::sleep, spawn};

// fn main() {
//     // 创建一个消息通道, 返回一个元素: (发送者, 接收者)
//     let (tx, rx) = mpsc::channel();

//     // 创建线程发送消息
//     thread::spawn(move || {
//         println!("send before");
//         // 发送一个数字1, send方法返回Result<T, E>, 通过unwrap进行快速错误处理;
//         tx.send(1).unwrap();

//         println!("send after");

//         // 下面代码将报错, 因为编译器自动推导出通过传递的值是i32类型, 那么Option<i32>类型将不匹配
//         // tx.send(Some(1)).unwrap()
//     });

//     // 在主线程下接收子线程发送的消息并输出
//     println!("receive before");
//     // println!("receive {}", rx.recv().unwrap());
//     println!("receive {}", rx.try_recv().unwrap());
//     println!("receive after");
// }

// fn main() {
//     let (tx, rx) = mpsc::channel();
//     let tx1 = tx.clone();
//     thread::spawn(move || {
//         tx.send("ho from raw tx".to_string()).unwrap();
//     });

//     thread::spawn(move || {
//         tx1.send("hi from cloned tx".to_string()).unwrap();
//     });

//     for received in rx {
//         println!("Got: {}", received);
//     }
// }

// fn main() {
//     let m = Mutex::new(1);
//     let a = m.lock().unwrap();
//     let a = m.lock().unwrap();
// }

// fn main() {
//     let flag = Arc::new(Mutex::new(false));
//     let cond = Arc::new(Condvar::new());
//     let cflag = flag.clone();
//     let ccond = cond.clone();

//     let hdl = thread::spawn(move || {
//         let mut lock = cflag.lock().unwrap();
//         let mut counter = 0;
//         while counter < 3 {
//             while !*lock {
//                 // wait方法会接收一个MutexGuard, 且它会自动的暂时释放这个所, 使其他线程可以拿到锁并进行数据更新
//                 // 同时当前线程在此处会被阻塞, 直到被其他地方notify之后, 他会将原本的MutexGuard还给我们, 即中心获取到了锁, 同时唤醒了此线程
//                 lock = ccond.wait(lock).unwrap();
//             }

//             *lock = false;
//             counter += 1;
//             println!("inner counter: {counter}");
//         }
//     });

//     let mut counter = 0;
//     loop {
//         thread::sleep(Duration::from_micros(1000));
//         *flag.lock().unwrap() = true;
//         counter += 1;
//         if counter > 3 {
//             break;
//         }

//         println!("outside counter: {counter}");
//         cond.notify_one();
//     }

//     hdl.join().unwrap();
//     println!("{flag:?}");
// }

use tokio::{self, sync::Semaphore};

#[tokio::main]
async fn main() {
    let semaphore = Arc::new(Semaphore::new(3));
    let mut join_handlers = Vec::new();

    for _ in 0..5 {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        join_handlers.push(tokio::spawn(async move {
            // 在这里执行任务...
            drop(permit);
        }));
    }

    for handle in join_handlers {
        handle.await.unwrap();
    }
}
