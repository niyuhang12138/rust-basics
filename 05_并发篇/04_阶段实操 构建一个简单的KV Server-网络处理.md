# 阶段实操: 构建一个简单的KV Server - 网络处理

经历了基础篇和进阶篇中的构建和优化, 到现在我们的KV Server核心功能已经比较完善了, 不知道你与没有注意, 之前一直在使用的一个神秘的库async-prost库, 我们神奇的完成了TCP frame的封包和解包, 是怎么完成的

async-prost是仿照Jonhoo的async-bincode做的一个处理protobuf frame的库, 它可以和各种网络协议适配, 包括TCP / WebSocket / HTTP2等, 由于考虑通用性, 它的抽象级别更高, 用了大量的泛型参数, 主流程如下图:

![image-20241220151610900](assets/image-20241220151610900.png)

主要的思路就是在序列化数据的时候, 添加一个头部的frame的长度, 反序列化的时候先读出头部, 获得长度, 在读取相应的数据

今天的挑战是在上一次完成KV Server的基础上, 试着不依赖async-prost, 自己处理封包和解包的逻辑, 如果你掌握了这个能力, 配合protobuf, 就可以设计出任何可以承载实际业务的协议了

## 如何定义协议的Frame?

protobuf帮我们解决了协议消息如何定义的问题, 然而一个消息和另一个消息如何区分是个伤脑筋的问题, 我们需要定义合适的分隔符

分隔符+消息数据, 就是一个Frame, 之前我们见过如何界定一个frame

很多基于TCP的协议会使用`\r\n`做分隔符, 比如FTP; 也有使用消息长度的做分隔符的, 比如gRPC; 还有混用两者的, 比如Redis的RESP; 更加复杂的如HTTP, header之间使用`\r\n`分隔, header / body之间使用`\r\n\t\n`, header中会提供body的长度等等

`\r\n`这样的分隔符, 适合协议报文是ASCII数据; 而通过长度进行分隔适合协议报文是二进制数据, 我们的KV Server承载的是protobuf是二进制, 所以在payload之前放一个长度, 来作为frame的分割

这个长度取什么大小呢? 如果使用两个字节, 那么payload最大是64k; 如果使用4个字节, payload可以到达4G, 一般的应用取四个字节就足够了, 如果你想要更灵活些, 也可以使用varint

tokio有个tokio-util库, 已经帮我们处理了和frame相关的封包解包的主要需求, 包括LineDelimited(处理`\r\n`分隔符)和LengthDelimited(处理长度分割符), 我们可以使用它的LengthDelimitedCodec尝试一下

```rust
[package]
name = "kv"
version = "0.1.0"
edition = "2021"

[dependencies]
bytes = "1" # 高效处理网络 buffer 的库
dashmap = "6.1.0" # 并发 HashMap
http = "1.2" # 我们使用 HTTP status code 所以引入这个类型库
prost = "0.9" # 处理 protobuf 的代码
rocksdb = "0.22.0"
sled = "0.34.7"
thiserror = "2.0.6" # 错误定义和处理
tracing = "0.1" # 日志处理
update = "0.0.0"

[dev-dependencies]
anyhow = "1" # 错误处理
async-prost = "0.3" # 支持把 protobuf 封装成 TCP frame
futures = "0.3" # 提供 Stream trait
tempfile = "3.14.0"
tokio = { version = "1", features = ["rt", "rt-multi-thread", "io-util", "macros", "net" ] } # 异步网络库
tokio-util = { version = "0.7.13", features = ["codec"] }
tracing-subscriber = "0.3" # 日志处理

[build-dependencies]
prost-build = "0.9" # 
```

然后创建`examples/server_with_codec.rs`文件, 添加如下代码:

```rust
use anyhow::Result;
use futures::{SinkExt, StreamExt};
use kv::{CommandRequest, MemTable, Service, ServiceInner};
use prost::Message;
use tokio::net::TcpListener;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let service: Service = ServiceInner::new(MemTable::new()).into();
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {addr}");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {addr:?} connected");
        let svc = service.clone();

        tokio::spawn(async move {
            let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
            while let Some(Ok(mut buf)) = stream.next().await {
                let cmd = CommandRequest::decode(&buf[..]).unwrap();
                info!("Got a new command: {cmd:?}");
                let res = svc.execute(cmd);
                buf.clear();
                res.encode(&mut buf).unwrap();
                stream.send(buf.freeze()).await.unwrap();
            }
            info!("Client {addr:?} disconnected");
        });
    }
}
```

你可以对比一下它和之前的`examples/server.rs`的差别, 主要改动了和这一行

```rust
// let mut stream = AsyncProstStream::<_, CommandRequest, CommandResponse, _>:
let mut stream = Framed::new(stream, LengthDelimitedCodec::new());
```

你是不是有些疑惑, 为什么客户端没做什么修改也能和服务器通信? 那是因为在目前的使用场景下, 使用AsyncProst的客户端兼容LengthDelimitCodec

## 如何撰写Frame的代码?

LengthDelimitedCodec非常好用, 它的代码也并不复杂, 非常建议你有空研究一下, 既然这一讲主要围绕网络开发来讲, 那么哦我们也来查实一下撰写字节的对Frame处理的代码把

按照前面的分析, 我们在protobuf payload前加一个四个字节的长度, 这样对端读取数据的时候, 可以先读四个数据, 然后根据读到的长度, 进一步满足这个长度的数据, 之后就可以用相应的数据结构解包了

为了更贴近实际, 我们把四字节长度的最高位拿出来作为是否压缩的信号, 如果设置了, 代表后续的payload是gzip压缩过的protobuf, 否则直接是protobuf:

![image-20241220155307130](assets/image-20241220155307130.png)

按照惯例, 还是先来定义这个逻辑的trait

```rust
use bytes::BytesMut;
use prost::Message;

use crate::KvError;

pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// 把一个Message encode成一个frame
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), KvError>;

    /// 把一个完成的frame decode成一个Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, KvError>;
}
```

定义了两个方法

- encode_frame可以把诸如CommandRequest这样的消息封装成一个frame, 写入传进来的BytesMut
- decode_frame可以把收到的一个完整的, 放在BytesMut中的数据, 封装成诸如CommandRequest这样的消息

如果要实现这个trait, Self需要实现了prost::Message, 大小是固定的, 并且实现了Default(prost的需求)

好我们在写实现代码, 首先创建`src/network`目录, 添加文件

因为要处理gzip压缩, 还需要在toml文件中引入flate2, 因为今天这一讲引入了网络相关的操作主句结构, 我们需要把tokio从dev-dependencies移动到dependencies里, 为了简单起见, 就用full features

```toml
[package]
name = "kv"
version = "0.1.0"
edition = "2021"

[dependencies]
bytes = "1" # 高效处理网络 buffer 的库
dashmap = "6.1.0" # 并发 HashMap
flate2 = "1.0.35"
http = "1.2" # 我们使用 HTTP status code 所以引入这个类型库
prost = "0.9" # 处理 protobuf 的代码
sled = "0.34.7"
thiserror = "2.0.6" # 错误定义和处理
tokio = { version = "1", features = ["full"] }
tracing = "0.1" # 日志处理
update = "0.0.0"
tracing-subscriber = "0.3" # 日志处理
anyhow = "1" # 错误处理

[dev-dependencies]
async-prost = "0.3" # 支持把 protobuf 封装成 TCP frame
futures = "0.3" # 提供 Stream trait
tempfile = "3.14.0"
tokio-util = { version = "0.7.13", features = ["codec"] }

[build-dependencies]
prost-build = "0.9" # 
```

然后在`src/network/frame.rs`中添加实现代码

```rust
use std::io::{Read, Write};

use bytes::{Buf, BufMut, BytesMut};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use prost::Message;
use tracing::debug;

use crate::{CommandRequest, CommandResponse, KvError};

/// 长度整个占用4个字节
pub const LEN_LEN: usize = 4;

/// 长度占31bit, 所以最大的frame是2G
pub const MAX_FRAME: usize = 2 * 1024 * 1024 * 1024;

/// 如果payload超过1436字节 就压缩
pub const COMPRESSION_LIMIT: usize = 1436;

/// 代表压缩的bit(整个4字节的最高位)
pub const COMPRESSION_BIT: usize = 1 << 31;

pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// 把一个Message encode成一个frame
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), KvError> {
        let size = self.encoded_len();

        if size > MAX_FRAME {
            return Err(KvError::FrameError);
        }

        // 我们先写入长度, 如果需要压缩, 在重写压缩后的长度
        buf.put_u32(size as _);

        if size > COMPRESSION_LIMIT {
            let mut buf1 = Vec::with_capacity(size);
            self.encode(&mut buf1)?;

            // BytesMut支持逻辑上的split(之后还能unsplit)
            // 所以我们先把长度中4字节长度, 清除
            let payload = buf.split_off(LEN_LEN);
            buf.clear();

            // 处理gzip压缩, 具体可以参考flate2文档
            let mut encoder = GzEncoder::new(payload.writer(), Compression::default());
            encoder.write_all(&buf1[..])?;

            // 压缩完成后, 从gzip encode中吧BytesMut在拿回来
            let payload = encoder.finish()?.into_inner();
            debug!("Encode a frame: size: {}({})", size, payload.len());

            // 写入压缩后的长度
            buf.put_u32((payload.len() | COMPRESSION_BIT) as _);

            // 把BytesMut在合并回来
            buf.unsplit(payload);

            Ok(())
        } else {
            self.encode(buf)?;
            Ok(())
        }
    }

    /// 把一个完成的frame decode成一个Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, KvError> {
        // 先取四个字节
        let header = buf.get_u32() as usize;
        let (len, compressed) = decode_header(header);
        debug!("Got a frame: msg len {}, compressed {}", len, compressed);

        if compressed {
            // 解压缩
            let mut decoder = GzDecoder::new(&buf[..len]);
            let mut buf1 = Vec::with_capacity(len * 2);
            decoder.read_to_end(&mut buf1)?;
            buf.advance(len);

            // decode成相应详细
            Ok(Self::decode(&buf1[..buf1.len()])?)
        } else {
            let msg = Self::decode(&buf[..len])?;
            buf.advance(len);
            Ok(msg)
        }
    }
}

impl FrameCoder for CommandRequest {}

impl FrameCoder for CommandResponse {}

pub fn decode_header(header: usize) -> (usize, bool) {
    let len = header & !COMPRESSION_BIT;
    let compressed = header & COMPRESSION_BIT == COMPRESSION_BIT;
    (len, compressed)
}
```

这段代码本身不难理解, 我们直接为FrameCoder提供了缺省实现, 然后CommandRequest / CommandResponse做了空实现, 其中使用了之前介绍过的bytes库里的BytesMut, 以及新引入的GzEncoder / GzDecoder, 最后还写了一个辅助函数decode_header, 让decode_frame的代码更直观一些

如果你有些疑惑为什么COMPRESSION_LIMIT设成1436?

这是因为以太网的MTU是1500, 除去IP头20字节, TCP头20字节, 还剩下1460; 一般TCP包会包含一些Option(比如timestamp), IP包也可能包含, 所以我们预留20字节; 在减去4字节的长度就是1436, 不用分片的最大消息长度, 如果大于这个, 很可能会导致分片, 我们就干脆压缩一下

现在CommandRequest / CommandResponse就可以做frame级别的处理了, 我们写一些测试验证是否可以工作, 还是在这个文件中, 添加测试代码

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use bytes::Bytes;

    #[test]
    fn command_request_encode_decode_should_work() {
        let mut buf = BytesMut::new();

        let cmd = CommandRequest::new_hdel("t1", "k1");

        cmd.encode_frame(&mut buf).unwrap();

        // 最高位没设置
        assert_eq!(is_compressed(&buf), false);

        let cmd1 = CommandRequest::decode_frame(&mut buf).unwrap();
        assert_eq!(cmd, cmd1);
    }

    #[test]
    fn command_response_encode_decode_should_work() {
        let mut buf = BytesMut::new();

        let values: Vec<Value> = vec![1.into(), "hello".into(), b"data".into()];

        let res: CommandResponse = values.into();

        res.encode_frame(&mut buf).unwrap();

        // 最高位长度没设置
        assert!(!is_compressed(&buf));

        let res1 = CommandResponse::decode_frame(&mut buf).unwrap();

        assert_eq!(res, res1);
    }

    fn is_compressed(data: &[u8]) -> bool {
        if let [v] = data[..1] {
            v >> 7 == 1
        } else {
            false
        }
    }
}
```

这个测试代码里面有从`[u8; N]`到Value`(b"data".into())`以及从Bytes到Value的转换, 所以我们需要在`src/pb/mod.rs`里添加From trait的相应实现

```rust
impl<const N: usize> From<&[u8; N]> for Value {
    fn from(buf: &[u8; N]) -> Self {
        Bytes::copy_from_slice(&buf[..]).into()
    }
}
impl From<Bytes> for Value {
    fn from(buf: Bytes) -> Self {
        Self {
            value: Some(value::Value::Binary(buf)),
        }
    }
}
```

运行cargo test, 所有测试都可以通过

到这里, 我们就完成了Frame的序列化(encode_frame)和反序列化(decode_frame), 并且用测试确保了它的正确性, 做网络开发的是哦户, 要尽可能的实现逻辑和IO的分离, 这样有助于可测试性以及应对IO层的变更, 目前这个代码没有触及任何的socket IO相关的内存, 只是纯逻辑, 接下来我们要将它和哦我们用于处理服务器客户端的TcpStream练习起来

再进一步写网络相关的代码之前, 还有一个问题需要解决: docode_frame函数使用的BytesMut, 是如何从socket里拿出来的, 显然先读四个字节, 取出长度N, 然后在读N个字节, 这个细节和frame关系很大, 所以还需要在`src/network/frame.rs`里写个辅助函数read_frame

```rust
/// 从stream中读取一个完整的stream
pub async fn read_frame<S>(stream: &mut S, buf: &mut BytesMut) -> Result<(), KvError>
where
    S: AsyncRead + Unpin + Send,
{
    let header = stream.read_u32().await? as usize;
    let (len, _compressed) = decode_header(header);
    // 如果美欧那么大的内存, 就分配至少一个frame的内存, 保证它可用
    buf.reserve(LEN_LEN + len);
    buf.put_u32(header as _);
    // advance_mut是unsafe的原因是, 从当前我盒子pos到pos + len
    // 这段内存目前没有初始化, 我们就是为了reserve这段内存, 然后从stream里读取, 读取完, 它就是初始化的, 所以我们这么用是安全的
    unsafe { buf.advance_mut(len) }

    stream.read_exact(&mut buf[LEN_LEN..]).await?;

    Ok(())
}
```

在写read_frame的时候, 我们不希望它只能用于TcpStream, 这样太不灵活了, 所以用了泛型参数S, 要求传入S必须满足AsyncRead + Unpin + Send, 我们来看看这三个约束

AsyncRead是tokio下的一个trait, 用于做异步读取, 它有一个方法poll_read

```rust
pub trait AsyncRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>
    ) -> Poll<Result<()>>;
}
```

一旦某个数据结构实现了AsyncRead, 它就可以使用AsyncReadExt提供的多达29个辅助方法, 这是因为任何实现了AsyncRead的数据结构, 都自动实现了AsyncReadExt:

```rust
impl<R: AsyncRead + ?Sized> AsyncReadExt for R {}
```

我们虽然还没有正式学怎么做异步处理, 但是之前看到了很多Async / await的代码

异步处理, 目前可以把它想象成一个内部状态机的数据结构, 异步运行时根据需要不断地做poll操作, 直到它返回Poll::Ready, 说明得到了处理结果; 如果它返回了Poll:::Pending, 说明目前无法继续, 异步运行时将其挂起, 等下次某个事件将这个任务唤醒

对于Socket来说, 读取socket局势不断poll_read的过程, 直到读到了满足的ReadBuf需要的内容

至于Send的约束, 很好理解, S需要能在不同线程间移动所有权, 对于Unpin约束, 未来在将Future的时候再具体说, 现在你就权记住, 如果一个编译器抱怨一个泛型参数cannot be unpinned, 一般来说这个泛型参数需要加Unpin的约束

既然有写了一些带阿米, 我们需要为其撰写相应的测试, 但是要测read_frame函数, 需要一个支持AsyncRead的数据结构, 虽然TcpStream支持它, 但是我们不应该在单元测试中引入太过于复杂的行为, 为了测试read_frame而建立TCP连接, 显然没有必要, 怎么办?

在之前聊过测试代码和产品代码同等重要性, 所以在开发中, 也要为测试代码创还能合适的生态环境, 让测试简洁, 可读性强, 所我们就创建一个简单的数据结构, 使其实现AsyncRead, 这样就可以单元测试read_frame了

在测试中加入一下代码:

```rust
struct DummyStream {
    buf: BytesMut,
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
```

因为只需要保证AsyncRead接口的正确性, 所以不需要太复杂的逻辑, 我们就放一个buffer, poll_read需要读多大的数据, 我们就给多大的数据, 有了这个DummyStream, 就可以测试read_frame了

```rust
#[tokio::test]
async fn read_frame_should_work() {
    let mut buf = BytesMut::new();
    let cmd = CommandRequest::new_hdel("t1", "k1");
    cmd.encode_frame(&mut buf).unwrap();
    let mut stream = DummyStream { buf };

    let mut data = BytesMut::new();
    read_frame(&mut stream, &mut data).await.unwrap();

    let cmd1 = CommandRequest::decode_frame(&mut data).unwrap();

    assert_eq!(cmd, cmd1);
}
```

## 让网络层可以像AsyncProst那样方便使用

现在我们的frame已经可以正常工作了, 接下来要构思一下, 服务端和客户端如何封装

对于服务器, 我们希望可以对accept下来的TcpStream提供一个process方法, 处理协议的细节

```rust
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:9527";
    let service: Service = ServiceInner::new(MemTable::new()).into();
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {}", addr);
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);
        let stream = ProstServerStream::new(stream, service.clone());
        tokio::spawn(async move { stream.process().await });
    }
}
```

这个process方法, 实际上就是对`examples/server.rs`中tokio::spawn里的while loop的封装

```rust
while let Some(Ok(cmd)) = stream.next().await {
    info!("Got a new command: {:?}", cmd);
    let res = svc.execute(cmd);
    stream.send(res).await.unwrap();
}
```

对于客户端, 我们希望直接execute一个命令, 就能得到一个结果:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:9527";
    // 连接服务器
    let stream = TcpStream::connect(addr).await?;
    let mut client = ProstClientStream::new(stream);
    // 生成一个 HSET 命令
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into);
    // 发送 HSET 命令
    let data = client.execute(cmd).await?;
    info!("Got response {:?}", data);
    Ok(())
}
```

这个execute, 实际上就是对examples/client.rs`中发送和接受代码的封装

```rust
client.send(cmd).await?;
if let Some(Ok(data)) = client.next().await {
    info!("Got response {:?}", data);
}
```

这样的代码, 看起来很简洁, 维护起来也很方便

先看服务器处理一个TcpStream的数据结构, 它需要包含TcpStream, 还有我们之前创建用于处理客户端命令的Service, 所以让服务器处理TcpStream的结构包含这两部分:

```rust
pub struct ProstServerStream<S> {
  inner: S,
  service: Service,
}
```

而客户端处理TcpStream的结构就只需要包含TcpStream

```rust
pub struct ProstClientStream<S> {
  inner: S,
}
```

这里, 依旧使用了泛型参数S, 未来如果要支持WebSocket, 或者在TCP之上支持TLS, 他都可以让我们无需改变这一层代码

接下来就是具体的实现了, 有了frame的封装, 服务器的process方法和客户端的execute方法都很容易实现, 我们直接在`src/network/mod.rs`中添加完整代码:

```rust
use anyhow::Result;
use bytes::BytesMut;
pub use frame::*;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tracing::info;

use crate::{CommandRequest, CommandResponse, KvError, Service};

mod frame;

/// 处理服务器端某个accept下来的socket读写
pub struct ProstServerStream<S> {
    inner: S,
    service: Service,
}

impl<S> ProstServerStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S, service: Service) -> Self {
        Self {
            inner: stream,
            service,
        }
    }

    pub async fn process(mut self) -> Result<(), KvError> {
        while let Ok(cmd) = self.recv().await {
            info!("Got a new command: {cmd:?}");
            let res = self.service.execute(cmd);
            self.send(res).await?;
        }
        Ok(())
    }

    async fn send(&mut self, msg: CommandResponse) -> Result<(), KvError> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf)?;
        let encoded = buf.freeze();
        self.inner.write_all(&encoded[..]).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandRequest, KvError> {
        let mut buf = BytesMut::new();
        let stream = &mut self.inner;
        read_frame(stream, &mut buf).await;
        CommandRequest::decode_frame(&mut buf)
    }
}

/// 处理客户端socket读写
pub struct ProstClientStream<S> {
    inner: S,
}

impl<S> ProstClientStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self { inner: stream }
    }

    pub async fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, KvError> {
        self.send(cmd).await?;
        Ok(self.recv().await?)
    }

    async fn send(&mut self, msg: CommandRequest) -> Result<(), KvError> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf)?;
        let encoded = buf.freeze();
        self.inner.write_all(&encoded[..]).await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<CommandResponse, KvError> {
        let mut buf = BytesMut::new();
        let stream = &mut self.inner;
        read_frame(stream, &mut buf).await?;
        CommandResponse::decode_frame(&mut buf)
    }
}
```

这段代码并不难阅读, 基本上和frame的测试代码大同小异

当然, 我们还是需要写段代码来测试一些客户端和服务器的交互流程:

```rust
#[cfg(test)]
mod tests {

    use std::{net::SocketAddr, vec};

    use bytes::Bytes;
    use tokio::net::{TcpListener, TcpStream};

    use crate::{assert_res_ok, MemTable, ServiceInner, Value};

    use super::*;

    #[tokio::test]
    async fn client_server_basic_communication_should_work() -> anyhow::Result<()> {
        let addr = start_server().await?;

        let stream = TcpStream::connect(addr).await?;
        let mut client = ProstClientStream::new(stream);

        // 发送HSET等待回应
        let cmd = CommandRequest::new_hset("t1", "k1", "v1".into());
        let res = client.execute(cmd).await.unwrap();

        // 第一个HSET服务器应该返回None
        assert_res_ok(res, &[Value::default()], &[]);

        // 再发一个HGET
        let cmd = CommandRequest::new_hget("t1", "k1");
        let res = client.execute(cmd).await?;

        // 服务器应该返回上一次的结果
        assert_res_ok(res, &["v1".into()], &[]);

        Ok(())
    }

    #[tokio::test]
    async fn client_server_compression_should_work() -> anyhow::Result<()> {
      let addr = start_server().await?;

      let stream = TcpStream::connect(addr).await?;
      let mut client = ProstClientStream::new(stream);

      let v: Value = Bytes::from(vec![0u8; 16384]).into();
      let cmd = CommandRequest::new_hset("t2", "k2", v.clone());
      let res = client.execute(cmd).await?;

      assert_res_ok(res, &[Value::default()], &[]);

      let cmd = CommandRequest::new_hget("t2", "k2");
      let res = client.execute(cmd).await?;

      assert_res_ok(res, &[v], &[]);

      Ok(())
    }

    async fn start_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let service: Service = ServiceInner::new(MemTable::new()).into();
                let server = ProstServerStream::new(stream, service);
                tokio::spawn(server.process());
            }
        });
        Ok(addr)
    }
}
```

## 正式创建Kv-server和kv-client

我们之前写了很多代码, 真正可运行的都是server / client都是examples下的代码, 现在我们终于要正式创建Kv-server / Kv-client了

首先在Cargo.toml文件中, 加入两个可执行文件: kvs和kvc, 还需要把一些依赖移动到dependencies下

```rust
[package]
name = "kv"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "kvs"
path = "src/server.rs"

[[bin]]
name = "kvc"
path = "src/client.rs"

[dependencies]
bytes = "1" # 高效处理网络 buffer 的库
dashmap = "6.1.0" # 并发 HashMap
flate2 = "1.0.35"
http = "1.2" # 我们使用 HTTP status code 所以引入这个类型库
prost = "0.9" # 处理 protobuf 的代码
sled = "0.34.7"
thiserror = "2.0.6" # 错误定义和处理
tokio = { version = "1", features = ["full"] }
tracing = "0.1" # 日志处理
update = "0.0.0"
tracing-subscriber = "0.3" # 日志处理
anyhow = "1" # 错误处理

[dev-dependencies]
async-prost = "0.3" # 支持把 protobuf 封装成 TCP frame
futures = "0.3" # 提供 Stream trait
tempfile = "3.14.0"
tokio-util = { version = "0.7.13", features = ["codec"] }

[build-dependencies]
prost-build = "0.9" # 
```

然后创建`src/client.rs`和`src/server.rs`, 分别写入以下代码:

**client**

```rust
use anyhow::Result;
use kv::{CommandRequest, ProstClientStream};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";

    let stream = TcpStream::connect(addr).await?;

    let mut client = ProstClientStream::new(stream);

    // HSET
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into());

    // 发送命令
    let data = client.execute(cmd).await?;

    info!("Got response {data:?}");

    Ok(())
}
```

**servr**

```rust
use anyhow::Result;
use kv::{MemTable, ProstServerStream, Service, ServiceInner};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.0.1:9527";

    let service: Service = ServiceInner::new(MemTable::new()).into();

    let listener = TcpListener::bind(addr).await?;

    info!("Start listening on {addr}");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {addr:?} connected");
        let stream = ProstServerStream::new(stream, service.clone());
        tokio::spawn(async move { stream.process().await });
    }
}
```

这和之前的代码几乎一致, 不同的是, 我们使用了自己的frame处理方法

完成之后, 我们可以打开一个命令行窗口, 运行`RUST_LOG=info cargo run --bin kvs --quiet`, 然后在另一个命令行窗口运行`RUST_LOG=info cargo run --bin kvc --quiet`, 此时服务器和客户端都收到了彼此的请求和响应, 并且处理正常

## 小结

网络开发时Rust下一个很重要的应用场景, tokio为我们提供了很棒的异步网络开发支持

在开发网络协议的时候, 你要确定你的frame如何封装, 一般来说, 长度 + protobuf足以应付绝大多数的场景, 这一讲我们虽然详细解析介绍了自己改如何处理长度封装frame的方法, 其实tokio-util提供了LengthDelimitCodec, 可做完成今天frame部分的处理, 如果自己撰写网络程序, 可以直接使用它

在网络开发的时候, 如何做单元测试是一大痛点, 我们可以根据其实现的接口, 围绕着接口来构建测试数据结构, 比如TcpStream实现了AsyncRead / AsyncWrite, 考虑简洁和可读, 为了测试read_frame, 我们构建了DummyStream来协助测试, 你也可以用类似的方式处理你所做项目的测试需求

结构良好架构清晰的代码, 一定是很容易测试的代码, 纵观整个项目, 从CommandService triat和Storage trait, 一路到现在网络层的测试, 如果使用tarpaulin来看测试覆盖率, 你会发现, 有接近89%, 如果不算`src/server.rs`和`src/client.rs`的话, 有接近92%的测试覆盖率, 即便在生产环境的代码里, 这也算是很高质量的测试覆盖率了
