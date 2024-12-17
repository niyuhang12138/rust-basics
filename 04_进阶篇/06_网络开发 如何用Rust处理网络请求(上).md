# 网络开发: 如何使用Rust处理网络请求?

今天我们学习如何使用Rust做网络开发

在互联网时代, 谈到网络, 我们想到的首先是Web开发以及设计的部分HTTP协议,和WebSocket协议

之所说部分, 是因为很多协议考虑到的部分, 比如更新是的并发控制, 大多数Web开发者并不知道, 当谈论到gRPC时, 很多人就会认为这是比较神秘的底层协议, 其实只不过是HTTP/2下的一种对二进制格式的封装

所以对于网络开发, 这个非常宏大的议题, 我们当然是不可能, 也没有必要覆盖全部内容的, 今天我们就先简单聊聊网络开发的大全景图, 然后重点学习如何使用Rust标准库以及生态系统中的库来做网络处理, 包括网络连接, 网络数据处理的一些方法, 最后也会介绍几种典型的通讯模型的使用

我们先来简单回顾一个ISO/OSI七层模型以及对应的协议, 物理层主要根PHY芯片有关, 就不多提了

![image-20241217161633096](assets/image-20241217161633096.png)

七层模型中, 链路层和网络层一般构建在操作系统之中, 我们并不需要直接接触, 而表现层和应用层关系紧密, 所以在实现过程中, 大部分应用程序只关心网络层, 传输层, 应用层

网络层目前处于IPv4和IPv6分庭抗礼, IPv6还未完全对IPv4取而代之; 传输层除了对延迟非常敏感的应用(比如游戏), 绝大多数应用都使用TCP; 而在应用层, 对用户友好, 且对防火墙友好的HTTP协议家族, HTTP / WebSorcket / HTTP/2, 以及尚在草案中的HTTP/3, 在漫长的进化中, 脱颖而出, 成为应用程序的主流的选择

我们来看看Rust生态对网络协议的支持:

![image-20241217162234591](assets/image-20241217162234591.png)

Rust标准库提供了`std::net`, 为整个TCP/IP协议栈的使用提供了封装, 然而`std::net`是同步的, 所以, 如果你要构建一个高性能的异步网络, 可以使用tokio, tokio提供了和`std::net`几乎一致的封装, 一旦熟悉了`std::net`, `tokio::net`里的功能对于来说并不陌生, 我们先从`std::net`开始了解

## std::net

`std::net`下提供了处理TCP / UDP的数据而机构, 以及一些辅助结构:

- TCP: TcpListener / TCPStream, 处理服务器的监听和客户端的链接
- UDP: UdpSocket, 处理UDP socket
- 其他: IpAddr是IPv4和IPv6地址的封装; SocketAddr, 表示IP地址 + 端口的数据结构

助力主要介绍一些TCP的处理, 顺便会是用到IpAddr / SocketAddrt

### TcpListener / TcpStream

如果要创建一个TCP server, 我们可以使用TcpListener绑定某个端口, 然后用loop循环处理接收到的用户请求, 接收到请求后, 会得到一个TcpStream, 它实现了Read / Write trait, 可以像读写文件一项, 进行socket的读写

```rust
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn main() {
    let listener = TcpListener::bind("0.0.0.0:9527").unwrap();
    loop {
        let (mut stream, addr) = listener.accept().unwrap();
        println!("Accepted a new connection: {addr}");
        thread::spawn(move || {
            let mut buf = [0u8; 12];
            stream.read_exact(&mut buf).unwrap();
            println!("data: {:?}", String::from_utf8_lossy(&buf));
            // 一个写了17个字节
            stream.write_all(b"glad to meet you!").unwrap()
        });
    }
}
```

对于客户端, 我们可以使用`TcpStream::connect`得到一个TcpStream, 一旦客户端的请求被服务器接收, 就可以发送或者接收数据:

```rust
use std::{
    io::{Read, Write},
    net::TcpStream,
};

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:9527").unwrap();
    // 一个写了12个字节
    stream.write_all(b"hello world!").unwrap();

    let mut buf = [0u8; 17];
    stream.read_exact(&mut buf).unwrap();
    println!("data: {:?}", String::from_utf8_lossy(&buf));
}
```

在个例子中, 客户端在连接成功后, 会发送12个字节的hello world!给服务器, 服务器读取并回复, 客户端会尝试接收完成的, 来自服务器的17个字节的glad to meet you!

但是目前客户端都需要硬编码要接收数据的大小, 这样不够灵活, 后续我们会看到如何通过使用消息帧(frame)更好的处理

从客户端的代码可以看到, 我们无需显示的关闭TcpStream, 因为TcpStream的内部实现也处理了Drop trait, 使得其离开作用域时会自动关闭

但如果你去看TcpStream的文档, 会发现并没有实现Drop, 这是因为TcpStream内部包装了sys_common::net::TcpStream, 然后它有包装了Socket, 而Socket是一个平台相关的结构, 比如在Unix下实现是FileDesc, 然后他内部是一个OwnedFd, 最后会调用`liib::close(self.fs)`来关闭fd, 也就关闭了TcpStream

### 处理网络连接的一般方法

如果你使用了某个Web Framework处理Web流量, 那么无需关心网络连接, 框架会自动帮助你大点好一切, 你只需要关心某个路由或者某一个RPC的处理逻辑就可以了, 到哪如果你要在TCP纸上构建自己的协议, 那么你需要认真考虑如果妥善处理网路连接

我们在之前的listener代码中也看到了, 在网络处理的主循环中, 会不断accept一个新的连接

```rust
fn main() {
    ...
    loop {
        let (mut stream, addr) = listener.accept().unwrap();
        println!("Accepted a new connection: {}", addr);
        thread::spawn(move || {
            ...
        });
    }
}
```

但是处理连接的过程, 需要放在另一个线程或者另一个异步任务中, 而不要在主循环中处理, 因为这样会阻塞循环, 使其在处理完当前的连接前, 无法accept新的连接

所以, loop + spawn是处理网络连接的基本方式

![image-20241217165527093](assets/image-20241217165527093.png)

但是使用线程处理频繁连接和退出网络连接, 一来效率上有问题, 二来线程间如何共享公共的数据也让人头疼, 哦我们来详细的看看

### 如果处理大量连接?

如果不断的创建线程, 那么当连接一高, 就容易把系统中可用的线程资源吃光, 此外, 因为线程的调度是操作系统完成的, 每次调度都要经历一个复杂的, 不那么高效的save and load的上下文切换的过程, 所以如果使用线程, 那么在早遇到C10K的瓶颈, 也就是连接数到万这个借呗, 系统就会遇到资源和算力的双重瓶颈

从资源的角度, 过多的线程占用过多的内存, Rust缺省的栈大小是2M, 10k连接机会占用20G(当然缺省栈大小也可可以根据需要修改); 从算力的角度, 太多的线程在连接数据到达时, 会来来回回切换线程, 导致CPU过分忙碌, 无法处理更多的连接请求

所以, 对于潜在有大量连接的网络服务, 使用线程不是一个好的方式

如果要突破C10K的瓶颈达到C10M, 我们就只能使用在用户态的协程来处理, 要么是类似Erlang/Golang那样的有效协程(stackful coroutine), 要么是类似Rust异步处理这样的无效协程(stackless coroutine)

所以在Rust下大部分处理网络相关的代码中, 你会看到, 很少直接有用std::net进行处理的, 大部分都是使用某个异步网络运行时, 比如tokio

### 如何处理共享信息?

第二个问题, 在构建服务器时, 我们总会有一些共享的状态供所有的连接使用, 比如数据库的连接, 对于这样的场景, 如果希望共享数据不需要修改, 我们可以考虑使用`Arc<T>`, 如果需要修改, 可以使用`Arc<RwLock<T>>`

![image-20241217170714102](assets/image-20241217170714102.png)

但使用锁, 就意味着一旦在关键路径上需要访问被锁住的资源, 整个系统的吞吐量都会收到很大的影响

一种思路是, 我们把锁的粒度降低, 这样冲突就会见啥, 比如在KV Server中, 我们把key哈希一下摸N, 讲不通的key分摊到N个memory store中, 这样所的粒度就降低到之前的1/N个;额

![image-20241217170935945](assets/image-20241217170935945.png)

另一种思路是我们改变共享资源的访问方式, 使其只被一个特定的线程访问; 其他线程或者协程只能通过给其发送消息的方式与之交互, 如果你用Erlang / Golang, 这种方式你应该不陌生, 在Rust下, 可以使用channel数据结构

![image-20241217171803862](assets/image-20241217171803862.png)

Rust下的channel, 无论是标准库还是第三方库, 都有非常棒的实现, 同步的channel的有标准库的mpsc:channel和第三方库crossbeam_channel, 异步channel有tokio下的mpsc:channel, 以及flume