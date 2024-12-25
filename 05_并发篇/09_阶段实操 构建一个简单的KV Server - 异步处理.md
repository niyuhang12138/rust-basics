# 阶段实操: 构建一个简单的KV Server - 异步处理

到目前为止, 我们已经一起完成了一个相对完善的KV Server, 还记得是怎么一步步的构建这个服务的么?

基础篇学完, 我们搭好了KV Server的基础功能, 构造了客户端和服务器间交互的protobuf, 然后设计了CommandService和Storage trait, 分别处理客户端命令和存储

在进阶篇掌握了trait的实战使用技巧之后, 哦我们进一步构造了Service数据结构, 接收CommandResponse, 根据其类型调用相应的CommandService处理, 并做合适的事件通知, 最后返回CommandResponse

但所有这一切都发生在同步的世界: 不管是数据是怎么获得的, 数据已经在哪里, 我们需要做的就是把一种数据类型转换为另一种数据类型的运算而已:

之后哦我们涉足网络世界, 为KV Server构造了自己的frame: 一个包含长度和是否压缩的信息的4字节的头, 以及实际的payload; 还设计了一个FrameCoder来对frame进行封包和拆包, 这为接下来构造网络接口打下了坚实的基础, 考虑到网络安全, 我们提供了TLS的支持

在构建ProstStream的时候, 我们开始处理异步: ProstStream内部的stream需要支持AsyncRead + AsyncWrite, 这可以让ProstStream适配包括TcpStream和TlsStream在内的一切实现了AsyncRead和AsyncWrite的异步网络接口

至此, 我们打通了从远端得到了一个命令, 历经TCP, TLS, 然后被FrameCoder解出来一个CommandRequest, 交由Service来处理的过程, 把同步世界和异步世界连接起来的, 就是ProstServiceStream这个结构

这个从收到处理到处理完成后发包的完整流程和系统结构, 可以看下图:

![image-20241224153833699](D:/Users/ASUS/Documents/WeChat Files/wxid_5x2xi7n5xdxe22/FileStorage/File/2024-12/assets/image-20241224153833699.png)

## 今天做点什么?

虽然我们很高就已经撰写了不少异步和异步有关的代码, 但是最能体现Rust异步本质的poll, poll_read, poll_next这样的处理函数还没有怎么写过, 之前测试异步的read_frame写过一个DummyStream, 算是体验了一下底层的异步处理函数的复杂接口, 不过在DummyStream中, 我们并没有做任何复杂的动作:

```rust
struct DummyStream {
    buf: BytesMut,
}
impl AsyncRead for DummyStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        // 看看 ReadBuf 需要多大的数据
        let len = buf.capacity();
        // split 出这么大的数据
        let data = self.get_mut().buf.split_to(len);
        // 拷贝给 ReadBuf
        buf.put_slice(&data);
        // 直接完工
        std::task::Poll::Ready(Ok(()))
    }
}
```

上一讲我们学习了异步IO, 这堂课我们就学以致用, 对现有的代码做些重构, 让核心的ProstStream更符合Rust的异步IO接口逻辑, 具体要做点什么呢?

看之前写的ProstServerStream的process函数, 比较一下它和async_prost库的AsyncProst的调用逻辑

```rust
// process() 函数的内在逻辑
while let Ok(cmd) = self.recv().await {
    info!("Got a new command: {:?}", cmd);
    let res = self.service.execute(cmd);
    self.send(res).await?;
}
// async_prost 库的 AsyncProst 的调用逻辑
while let Some(Ok(cmd)) = stream.next().await {
    info!("Got a new command: {:?}", cmd);
    let res = svc.execute(cmd);
    stream.send(res).await.unwrap();
}
```

可以看到由于AsyncProst实现了Stream和Sink, 能更加自然的调用StreamExt trait的next方法和SinkExt trait方法, 来处理数据的收发, 而ProstServerStream则自己额外实现了函数的recv和send

虽然从代码对比的角度, 这两端代码几乎一样, 但未来的可拓展性和整个异步生态的融洽性上, AsyncProst还是更胜一筹

所以今天我们就构造一个ProstStream结构, 让它实现Stream和Sink这两个trait, 然后让ProstServerStream和ProstClientStream使用它

## 创建ProstStream

在开始重构之前, 先来简单复习一下Stream trait和Sink trait:

```rust
// 可以类比 Iterator
pub trait Stream {
    // 从 Stream 中读取到的数据类型
    type Item;
    // 从 stream 里读取下一个数据
    fn poll_next(
        self: Pin<&mut Self>, cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>>;
}
//
pub trait Sink<Item> {
    type Error;
    fn poll_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), Self::Error>>;
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error>
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), Self::Error>>;
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Result<(), Self::Error>>;
}
```

那么ProstStream具体需要包含什么类型呢?

因为 的主要职责就是从底下的stream中读取或者发送数据, 所以一个支持AsyncRead和AsyncWrite的泛型参数S是必然需要的

另外Stream trait和Sink都个需要一个Item类型, 对于我们的系统来说, Item是CommandRequest或者CommandResponse, 但是为了灵活性, 我们可以用In和Out这两个泛型参数来表示

当然, 在处理Stream和Sink时还需要read buffer和write buffer

综上所述, 我们的ProstStream结构看上去是这个样子的

```rust
pub struct ProstStream<S, In, Out> {
    // innner stream
    stream: S,
    // 写缓存
    wbuf: BytesMut,
    // 读缓存
    rbuf: BytesMut,
}
```

然而, Rust不允许数据结构有超出需要的泛型参数, 我们可以使用PhantomData, 之前讲过它是一个零字节大小的占位符, 可以让我们的数据结构携带未使用的泛型参数

好现在有足够的思路了, 我们创建`src/network/stream.rs`, 添加如下代码

```rust
use futures::{Sink, Stream};
use std::marker::PhantomData;
use tokio::io::{AsyncRead, AsyncWrite};

use bytes::BytesMut;

use crate::KvError;

use super::FrameCoder;

/// 处理KV Server prost frame的stream
pub struct ProstStream<S, In, Out> {
    // inner stream
    stream: S,
    // write cache
    wbuf: BytesMut,
    // read cache
    rbuf: BytesMut,

    // 类型占位符
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}

impl<S, In, Out> Stream for ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
    In: Unpin + Send + FrameCoder,
    Out: Unpin + Send,
{
    type Item = Result<In, KvError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

/// 当调用send的时候, 会把Out发出去
impl<S, In, Out> Sink<Out> for ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin,
    In: Unpin + Send,
    Out: Unpin + Send + FrameCoder,
{
    type Error = KvError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Out) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }
}
```

这段代码包含了为ProstStream实现Stream和Sink的骨架代码, 接下来我们就一个个的处理, 注意对于In和Out参数, 还未器约束了FrameCoder这样, 在实现中我们可以使用decode_frame和encode_frame来获取一个Item或者encode一个Item

## Stream的实现

先来实现Stream的poll_next方法

poll_next可以直接调用我们之前写好的read_frame, 然后再用decode_frame来解包

```rust
fn poll_next(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
) -> std::task::Poll<Option<Result<In, KvError>>> {
    // 上一次调用结束后rbuf应该Wie空
    assert!(self.rbuf.len() == 0);

    // 从rbuf中分离出rset(摆脱对self的引用)
    let mut rest = self.rbuf.split_off(0);

    // 使用read_frame来获取数据
    let fut = read_frame(&mut self.stream, &mut rest);
    ready!(Box::pin(fut).poll_unpin(cx))?;

    // 拿到一个frame的数据, 把buff合并回去
    self.rbuf.unsplit(rest);

    // 调用decode_frame获取解包之后的数据
    Poll::Ready(Some(In::decode_frame(&mut self.rbuf)))
}
```

这段代码不难理解, 但是中间这段需要解释一下:

```rust
// 使用 read_frame 来获取数据
let fut = read_frame(&mut self.stream, &mut rest);
ready!(Box::pin(fut).poll_unpin(cx))?;
```

因为poll_xxx方法已经是async/await底层API实现, 所以哦我们在poll_xxx方法中, 是不能使用异步函数的, 需要把它看做是一个future, 然后调用future的poll函数, 因为future是一个trait, 所以需要Box将其处理成一个对上的trait object, 这样就可以调用FutureExt的poll_unpin方法了, Box::pin会生成`Pin<Box>`

至于ready!宏, 它会在Pending的时, 直接return Pending, 而在Ready时, 返回Ready的值

```rust
#[macro_export]
macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            $crate::task::Poll::Ready(t) => t,
            $crate::task::Poll::Pending => return $crate::task::Poll::Pending,
        }
    };
}
```

Stream我们就实现好了

## Sink的实现

在写Sink, 看上去要实现好几个方法, 其实也并不复杂, 四个方法poll_ready, start_send, poll_flush, poll_close

poll_ready是做背压的, 你可以根据负载来决定要不要返回Poll::Ready, 对于我们网络层来说, 可以先不关心背压, 依靠操作系统的TCP协议栈提供背压处理即可, 所以这里直接返回Poll::Ready(Ok(()))即可,, 也就是说, 上层想写数据, 可以是随时写

```rust
fn poll_ready(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
) -> std::task::Poll<Result<(), KvError>> {
    Poll::Ready(Ok(()))
}
```

当poll_ready返回Ready后, Sink就走到了start_send, 我们在start_send里就把必要的数据准备好, 这里把item封包成字节流, 存入wbuf中

```rust
fn start_send(self: std::pin::Pin<&mut Self>, item: Out) -> Result<(), KvError> {
    let this = self.get_mut();
    item.encode_frame(&mut this.wbuf)?;
    Ok(())
}
```

然后早poll_flush中, 我们开始写数据, 这里需要记录当前写到哪里, 所以我们需要在ProstStream中加一个字段written, 记录写入了多少字节:

```rust
/// 处理KV Server prost frame的stream
pub struct ProstStream<S, In, Out> {
    // inner stream
    stream: S,
    // write cache
    wbuf: BytesMut,
    written: usize,
    // read cache
    rbuf: BytesMut,

    // 类型占位符
    _in: PhantomData<In>,
    _out: PhantomData<Out>,
}
```

有了这个written, 我们就可以循环写入了

```rust
fn poll_flush(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
) -> std::task::Poll<Result<(), KvError>> {
    let this = self.get_mut();

    // 循环写入stream中
    while this.written != this.wbuf.len() {
        let n = ready!(Pin::new(&mut this.stream).poll_write(cx, &this.wbuf[this.written..]))?;
        this.written += n;
    }

    // 清除wbuf
    this.wbuf.clear();
    this.written = 0;

    // 调用stream的pull_flush确保写入
    ready!(Pin::new(&mut this.stream).poll_flush(cx)?);
    Poll::Ready(Ok(()))
}
```

最后是Poll_close, 我们只需要调用stream的flush和shutdown方法, 确保数据写完是stream关闭

```rust
fn poll_close(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
) -> std::task::Poll<Result<(), KvError>> {
    // 调用stream的pull_flush确保写入
    ready!(self.as_mut().poll_flush(cx))?;

    // 调用stream的pull_close确保stream关闭
    ready!(Pin::new(&mut self.stream).poll_shutdown(cx))?;
    Poll::Ready(Ok(()))
}
```

## ProstStream的创建

我们的ProstStream目前液晶实现了Stream和Sink, 为了方便使用, 在构建一些辅助方法, 比如new

```rust
impl<S, In, Out> ProstStream<S, In, Out>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            written: 0,
            wbuf: BytesMut::new(),
            rbuf: BytesMut::new(),
            _in: PhantomData::default(),
            _out: PhantomData::default(),
        }
    }
}

// 一般来说, 我们的Stream是Unpin, 最好实现一下
impl<S, Req, Res> Unpin for ProstStream<S, Req, Res> where S: Unpin {}
```

此外, 我们还未器实现了Unpin trait, 这会给别人在使用你的代码时候带来很多方便, 一般来说, 为异步操作而诞生的额数据结构, 如果使用了泛型参数, 那么只要内部没有自引用数据, 就应该是Unpin

## 测试!

有道了重要的吃环境, 哦我们需要写点测试来确保ProstStream能正常工作, 因为之前在`src/network/frame.rs`中实现了AsyncRead, 我们只需要拓展它, 让它在实现AsyncWrite

为了让它可以被复用, 我们将其从frame.rs中移出来, 放在src/network/mod.rs中, 并修改成下面的样子

```rust
#[cfg(test)]
pub mod utils {
    use std::task::Poll;

    use bytes::BufMut;

    use super::*;

    pub struct DummyStream {
        pub buf: BytesMut,
    }

    impl AsyncRead for DummyStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            // 看看ReadBuf需要多大的数据
            let len = buf.capacity();

            // split 出这么大的数据
            let data = self.get_mut().buf.split_to(len);

            // 拷贝给ReadBuf
            buf.put_slice(&data);

            // 完工
            std::task::Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for DummyStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
            self.get_mut().buf.put_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<std::result::Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<std::result::Result<(), std::io::Error>> {
            Poll::Ready(Ok(()))
        }
    }
}
```

这样我们就给我们的Stream写个测试

```rust
#[cfg(test)]
mod tests {
    use super::super::utils::DummyStream;
    use super::*;
    use crate::CommandRequest;
    use anyhow::Result;
    use futures::{SinkExt, StreamExt};

    #[tokio::test]
    async fn prost_stream_should_work() -> Result<()> {
        let buf = BytesMut::new();
        let stream = DummyStream { buf };
        let mut stream = ProstStream::<_, CommandRequest, CommandRequest>::new(stream);
        let cmd: CommandRequest = CommandRequest::new_hdel("t1", "k1");
        stream.send(cmd.clone()).await?;
        if let Some(Ok(s)) = stream.next().await {
            assert_eq!(s, cmd);
        } else {
            assert!(false)
        }

        Ok(())
    }
}
```

## 使用ProstStream

接下来, 我们可以让ProstServerStream和ProstClientStream使用新定义的ProstStream, 你可以参考下面的对比

```rust
// 旧的接口
// pub struct ProstServerStream<S> {
// inner: S,
// service: Service,
// }

pub struct ProstServerStream<S> {
    inner: ProstStream<S, CommandRequest, CommandResponse>,
    service: Service,
}

// 旧的接口
// pub struct ProstClientStream<S> {
// inner: S,
// }

pub struct ProstClientStream<S> {
    inner: ProstStream<S, CommandResponse, CommandRequest>,
}
```

然后删除send / recv函数, 并修改process / execute函数使其使用next方法和send方法

```rust
/// 处理服务器端的某个 accept 下来的 socket 的读写
pub struct ProstServerStream<S> {
    inner: ProstStream<S, CommandRequest, CommandResponse>,
    service: Service,
}
/// 处理客户端 socket 的读写
pub struct ProstClientStream<S> {
    inner: ProstStream<S, CommandResponse, CommandRequest>,
}
impl<S> ProstServerStream<S>
where
S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S, service: Service) -> Self {
        Self {
            inner: ProstStream::new(stream),
            service,
        }
    }
    pub async fn process(mut self) -> Result<(), KvError> {
        let stream = &mut self.inner;
        while let Some(Ok(cmd)) = stream.next().await {
            info!("Got a new command: {:?}", cmd);
            let res = self.service.execute(cmd);
            stream.send(res).await.unwrap();
        }
        Ok(())
    }
}
impl<S> ProstClientStream<S>
where
S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ProstStream::new(stream),
        }
    }
    pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
        let stream = &mut self.inner;
        stream.send(cmd).await?;
        match stream.next().await {
            Some(v) => v,
            None => Err(KvError::Internal("Didn't get any response".into())),
        }
    }
}
```

我们重构了ProstServerStream和ProstClientStream的代码, 使其内部使用更符合futures库里的Stream / Sink trait的用法, 整体代码改动不小, 但是内部实现的变更并不影响其他部分

## 小结

在实际开发中, 进行重构来改善既有代码质量是必不可少的, 之前在开发KV Server的过程中, 我们在不断的进行一些小的重构

今天我们做了稍微大一些的重构, 为已有的代码提供更加符合异步IO接口的功能, 从对外的使用来说, 它并没有提供或者满足任何额外的需求, 但是从代码的质量和家督来说, 它使得我们的ProstStream可以更方便和直观地被其他接口调用, 也更容易跟整个Rust的现有生态结合

你可能会好奇, 为什么可以这么自然的机型代码重构, 这是因为我们有足够的单元测试覆盖

就像生物的进化一样, 好的代码是在良性的架构中不断演进出来的, 而在良性的重构实在优秀的单元测试的监管下, 使代码瞅着正确的方向迈出步伐, 在这里单元测试就是扮演者生物进化中的自然环境角色, 把重构过程中错误一一扼杀

