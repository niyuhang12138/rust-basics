# 阶段实操: 构建一个简单的KV Server - 高级trait技巧

今天我们就用之前1.0版简易的KV store来历练一把, 看看怎么把之前学到的知识融入到代码中

我们之间已经完成了1.0的功能, 但溜了两个小尾巴:

1. Storage trait的get_iter方法没有实现
2. Service的execute方法里有一些TODO, 需要处理事件的通知

## 处理Iterator

在开始撰写代码之前, 先把之前在`src/storage/mod.rs`里注释的测试加回来

```rust
#[test]
fn memtable_iter_should_work() {
    let store = MemTable::new();
    test_get_iter(store);
}
```

然后在`src/storage/memory.rs`里尝试实现它:

```rust
impl Storage for MemTable {
    ...
    fn get_iter(&self, table: &str) -> Result<Box<dyn Iterator<Item = Kvpair>>
    // 使用 clone() 来获取 table 的 snapshot
    let table = self.get_or_create_table(table).clone();
    let iter = table
    .iter()
    .map(|v| Kvpair::new(v.key(), v.value().clone()));
    Ok(Box::new(iter)) // <-- 编译出错
}
}
```

这里会报错`cannot return value referencing local variable table`, 原因是`table.iter`使用了table的引用, 我们返回iter, 但iter引用了作为局部变量table, 所以无法编译通过

此刻, 我们需要有一恶搞能够完全占有table的迭代器, Rust标准库中提供了一个IntoIterator trait, 它可以把数据结构的所有权转移到iterator中, 看它的声明:

```rust
pub trait IntoIterator {
    type Item;
    type IntoInter: Interator<Item = Self::Item>;
    
    fn into_iter(self) -> Self::IntoIter;
}
```

绝大多数的集合类型数据结构都实现了它, DashMap也实现了它, 所以我们可以用table.into_iter把table的所有权转移给iter`

```rust
fn get_iter(
    &self,
    table: &str,
) -> Result<Box<dyn Iterator<Item = crate::Kvpair>>, crate::KvError> {
    let table = self.get_or_create_table(table).clone();
    let iter = table.into_iter().map(|v| v.into());
    Ok(Box::new(iter))
}
```

这里又遇到了数据转换, 从DashMap中iterate出来的值需要转换成Kvpair, 我们依旧用into来完成这件事, 为此, 需要为Kvpair实现这个简单的From trait:

```rust
impl From<(String, Value)> for Kvpair {
    fn from(value: (String, Value)) -> Self {
        Kvpair::new(value.0, value.1)
    }
}
```

这个代码编译可以通过, 现在如果运行cargo test进行测试的话, 也可以通过

虽然这个代码可以通过测试, 并且本身也非常精简, 但是我们还是有必要思考一下, 如果想为更多的data store实现Storage trait, 都会怎样处理get_iter方法

我们会:

1. 拿到一个关于某个table的下的拥有所有权的Iterator
2. 对Iterator做map
3. 将map出来的每个Item转换成Kvpair

这里的第二步对于每个Storage trait的get_iter方法来说都是相同的, 有没有可能把它封装起来, 使得Storage trait的实现着只需要提供他们自己的拥有所有权Iterator, 并对Iterator里的Item类型提供`Into<Kvpair>`?

来尝试一下, 在`src/storage/mod.rs`, 构建一个StorageIter, 并实现Iterator trait:

```rust
/// 提供Storage Iterator, 这样trait的实现着只需要
/// 把他们的Iterator提供给StorageIter, 然后他们保证
/// next出来的类型实现了Into<Kvpair>即可
pub struct StorageIter<T> {
    data: T,
}

impl<T> StorageIter<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> Iterator for StorageIter<T>
where 
    T: Iterator,
    T::Item: Into<Kvpair>,
{
    type Item = Kvpair;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.next().map(|v| v.into())
    }
}
```

这样, 我们在`src/storage/memory.rs`里对get_iter`的实现, 就可以直接使用StorageIter了, 不过还要为DashMap的Iterator每次调用next得到的值(String, Value), 做个到Kvpair的转换

```rust
fn get_iter(
    &self,
    table: &str,
) -> Result<Box<dyn Iterator<Item = crate::Kvpair>>, crate::KvError> {
    let table = self.get_or_create_table(table).clone();
    let iter = StorageIter::new(table.into_iter());
    Ok(Box::new(iter))
}
```

我们可以再次使用cargo test测试, 同样通过

如果回顾刚才撰写的代码, 你可能会觉得我们辛辛苦苦的写了二十行代码, 创建了一个新的数据结构, 就是为了get_iter方法里的一行代码改的更漂亮, 为什么呢?

的确, 在这个例子中, 这个抽象的意义并不大, 但是, 如果刚才的那个步骤不是三步, 而是五步, 十步呢? 其中大量的步骤都是相同的, 也就是说, 我们每时间一个新的store, 就要撰写相同的代码逻辑, 那么, 这个抽象就非常有必要了

## 支持事件通知

我们再来看看事件通知, 在`src/service/mod.rs`中, 目前的execute方法还有很多TODO需要解决

```rust
pub fn execute(&self, cmd: CommandRequest) -> CommandResponse {
    debug!("Got request: {cmd:?}");

    // TODO: 发送on_received

    let res = dispatch(cmd, &self.inner.store);
    debug!("Execute response: {res:?}");

    // TODO: 发送on_executed

    res
}
```

为了解决这些TODO, 我们需要提供事件通知的机制:

1. 在创建Sevice时, 注册相应的事件处理函数
2. 在execute方法执行时, 做出相应的事件通知, 使得注册时间处理函数可以得到执行

先看看事件处理函数如何注册

如果想要能够注册, 那么倒推也就四, `Service/ServiceInner`数据结构就需要有地方能够承载事件注册函数, 可以尝试着把它加载ServiceInner结构里

```rust
/// Serviced的内部数据结构
pub struct ServiceInner<Store> {
    store: Store,
    on_received: Vec<fn(&CommandRequest)>,
    on_executed: Vec<fn(&CommandResponse)>,
    on_before_send: Vec<fn(&mut CommandResponse)>,
    on_after_send: Vec<fn()>,
}
```

按照之前的设计, 我们提供了四个事件:

1. on_received: 当服务器收到CommandRequest时触发
2. on_executed: 当服务器处理完CommandRequest得到CommandResponse时触发
3. on_before_send: 在服务器发送CommandResponse之前触发, 注意这个接口提供的是`&mut CommandResponse`, 这样的事件的创立者可以根据需要, 在发送前, 修改CommandResponse
4. on_after_send: 在服务器发送完CommandResponse后触发

在撰写事件注册的代码之前, 还是先写个测试, 从使用者的家督, 考虑如何进行测试

```rust
#[test]
fn event_registration_should_work() {
    fn b(cmd: &CommandRequest) {
        info!("Got {:?}", cmd);
    }
    fn c(res: &CommandResponse) {
        info!("{:?}", res);
    }
    fn d(res: &mut CommandResponse) {
        res.status = StatusCode::CREATED.as_u16() as _;
    }
    fn e() {
        info!("Data is sent");
    }

    let service = Service::new(MemTable::default())
    .fn_received(|_| {})
    .fn_received(b)
    .fn_executed(c)
    .fn_before_send(d)
    .fn_after_send(e)
    .into();

    let res = service.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
    assert_eq!(res.status, StatusCode::CREATED.as_u16() as _);
    assert_eq!(res.message, "");
    assert_eq!(res.values, vec![Value::default()]);
}
```

从测试代码可以看到, 我们希望通过ServiceInner结构, 不断调用fn_xxx方法, 为ServiceInner注册相应的事件处理函数, 添加完毕后, 我们再把ServiceInner转换成Service, 这是一种典型的构造者模式, 在很多Rust代码中, 都能看到它的神鹰

那么诸如fn_received这样的方法有什么魔力呢, 它为什么可以有做链式调用呢, 答案很简单, 他把self的所有权拿过来, 处理完毕后, 再返回self, 所以我们继续添加一下代码

```rust
impl<Store: Storage> ServiceInner<Store> {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            on_received: Vec::new(),
            on_executed: Vec::new(),
            on_before_send: Vec::new(),
            on_after_send: Vec::new(),
        }
    }

    pub fn fn_received(mut self, f: fn(&CommandRequest)) -> Self {
        self.on_received.push(f);
        self
    }

    pub fn fn_executed(mut self, f: fn(&CommandResponse)) -> Self {
        self.on_executed.push(f);
        self
    }

    pub fn fn_before_send(mut self, f: fn(&mut CommandResponse)) -> Self {
        self.on_before_send.push(f);
        self
    }

    pub fn fn_after_send(mut self, f: fn()) -> Self {
        self.on_after_send.push(f);
        self
    }
}
```

这样处理之后, Service之前的new方法就没有必要存在了, 可以把它删除, 同时, 我们需要为Service类型提供一个`From<ServiceInner>`的实现

```rust
impl<Store: Storage> From<ServiceInner<Store>> for Service<Store> {
    fn from(value: ServiceInner<Store>) -> Self {
        Self {
            inner: Arc::new(value)
        }
    }
}
```

目前, 代码中有几处使用了`Service::new`的地方需要改成使用`ServiceInner::new`

全部改动完成后, 代码可以编译通过

然后如果运行cargo test, 新加的测试会失败

```bash
test service::tests::event_registration_should_work ... FAILED
```

这是因为, 我们虽然完成了事件处理函数的注册, 但现在还没有发事件通知

另外因为我们的事件包裹不可变事件和可变事件, 所以事件通知需要把二者分开, 来定义两个trait: Notify和NotifyMut

```rust
/// 事件通知(不可变事件)
pub trait Notify<Arg> {
    fn notify(&self, arg: &Arg);
}

/// 事件通知(可变事件)
pub trait NotifyMut<Arg> {
    fn notify(&self, arg: &mut Arg);
}
```

由此我们可以特地为`Vec<fn(&Arg)>`和`Vec<fn<&mut (Arg)>`实现事件处理, 它们涵盖了目前支持的几种事件

```rust
/// 事件通知(不可变事件)
pub trait Notify<Arg> {
    fn notify(&self, arg: &Arg);
}

impl<Arg> Notify<Arg> for Vec<fn(&Arg)> {
    #[inline]
    fn notify(&self, arg: &Arg) {
        for f in self {
            f(arg)
        }
    }
}

/// 事件通知(可变事件)
pub trait NotifyMut<Arg> {
    fn notify(&self, arg: &mut Arg);
}

impl<Arg> NotifyMut<Arg> for Vec<fn(&mut Arg)> {
    fn notify(&self, arg: &mut Arg) {
        for f in self {
            f(arg)
        }
    }
}
```

Notify和NotifyMut trait实现好之后, 我们就可以修改execute方法了

```rust
pub fn execute(&self, cmd: CommandRequest) -> CommandResponse {
    debug!("Got request: {cmd:?}");

    self.inner.on_received.notify(&cmd);

    let mut res = dispatch(cmd, &self.inner.store);
    debug!("Execute response: {res:?}");

    self.inner.on_executed.notify(&res);
    self.inner.on_before_send.notify(&mut res);
    if !self.inner.on_before_send.is_empty() {
        debug!("Modified response: {res:?}")
    }

    res
}
```

现在, 响应的事件就尅被通知到相应的处理函数中了, 这个通知机制目前还是同步的函数调用, 未来如何需要, 我们可以将其改成消息传递, 进行异步处理

## 为持久化数据库实现Storage trait

到目前为止, 我们的KV store还都是一个在内存中的KV store, 一旦终止应用程序, 用户存储的所有key / value都会消失, 我们希望存储能够持久化

一个方案是为MemTable添加WAL和disk snapshot支持, 让用户发送的所有涉及更新的命令都按顺序存储在磁盘上, 同时定期做snapshot, 便于数据的快速恢复; 另一个方法是使用已有的KV store, 比如RocksDB或者sled

RocksDB是Facebook在Google的LevelDB基础上开发的嵌入式KV store, 用C++编写, 而sled是Rust社区里涌现的优秀的KV store, 对标RocksDB, 二者功能很类似, 从演示的角度来说, sled使用起来更简单, 更加适合今天的内容, 如果在生产环境中使用, RocksDB更加合适, 因为它在各种复杂的生产环境中经历了千锤百炼

所以我们今天尝试使用sled实现Storage trait, 让它能够适配我们的KV Server

首先在toml文件中引入sled

然后创建`src/storage/sleddb.rs`, 并添加如下代码:

```rust
use core::str;
use std::path::Path;

use sled::{Db, IVec};

use crate::{KvError, Kvpair, Storage, StorageIter, Value};

#[derive(Debug)]
pub struct SledDb(Db);

impl SledDb {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(sled::open(path).unwrap())
    }

    // 在sleddb里, 因为它可以scan_prefix, 我们用prefix
    // 来模拟一个table, 还可以用其他方法
    fn get_full_key(table: &str, key: &str) -> String {
        format!("{table}:{key}")
    }

    // 遍历table的key, 我们直接把prefix; 当成table
    fn get_table_prefix(table: &str) -> String {
        format!("{table}:")
    }
}

/// 把Option<Result<T, E>>flip转换成Result<Option<T>, E>
/// 从这个函数中, 我们看到函数式编程的优雅
fn flip<T, E>(x: Option<Result<T, E>>) -> Result<Option<T>, E> {
    x.map_or(Ok(None), |v| v.map(Some))
}

impl Storage for SledDb {
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, KvError> {
        let name = SledDb::get_full_key(table, key);
        let result = self.0.get(name.as_bytes())?.map(|v| v.as_ref().try_into());
        flip(result)
    }

    fn set(&self, table: &str, key: String, value: Value) -> Result<Option<Value>, KvError> {
        let key = key.as_str();
        let name = SledDb::get_full_key(table, key);
        let data: Vec<u8> = value.try_into()?;
        let result = self
            .0
            .insert(name.as_bytes(), data)?
            .map(|v| v.as_ref().try_into());
        flip(result)
    }

    fn contains(&self, table: &str, key: &str) -> Result<bool, KvError> {
        let name = SledDb::get_full_key(table, key);
        Ok(self.0.contains_key(name)?)
    }

    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, KvError> {
        let name = SledDb::get_full_key(table, key);
        let result = self
            .0
            .remove(name.as_bytes())?
            .map(|v| v.as_ref().try_into());
        flip(result)
    }

    fn get_all(&self, table: &str) -> Result<Vec<crate::Kvpair>, KvError> {
        let prefix = SledDb::get_table_prefix(table);
        let result = self.0.scan_prefix(prefix).map(|v| v.into()).collect();
        Ok(result)
    }

    fn get_iter(&self, table: &str) -> Result<Box<dyn Iterator<Item = crate::Kvpair>>, KvError> {
        let prefix = SledDb::get_table_prefix(table);
        let iter = StorageIter::new(self.0.scan_prefix(prefix));
        Ok(Box::new(iter))
    }
}

impl From<Result<(IVec, IVec), sled::Error>> for Kvpair {
    fn from(value: Result<(IVec, IVec), sled::Error>) -> Self {
        match value {
            Ok((k, v)) => match v.as_ref().try_into() {
                Ok(v) => Kvpair::new(ivec_to_key(k.as_ref()), v),
                Err(_) => Kvpair::default(),
            },
            _ => Kvpair::default(),
        }
    }
}

fn ivec_to_key(ivec: &[u8]) -> &str {
    let s = str::from_utf8(ivec).unwrap();
    let mut iter = s.split(":");
    iter.next();
    iter.next().unwrap()
}
```

这段代码主要就是在实现Storage trait, 每个方法都很简单, 就是在sled提供的功能上增加了一次封装, 如果你对代码中某个调用有顾虑, 可以参考sled的文档

在`src/storage/mod.rs`里引入sleddb, 我们就可以加上相关的测试, 测试新的Storage实现了

```rust
#[test]
fn sleddb_basic_interface_should_work() {
    let dir = tempdir().unwrap();
    let store = SledDb::new(dir);
    test_get_all(store)
}

#[test]
fn sleddb_iter_should_work() {
    let dir = tempdir().unwrap();
    let store = SledDb::new(dir);
    test_get_iter(store);
}
```

因为SledDB创建时需要指定一个目录, 所以要在测试中使用tempfile库, 它能让文件在测试件数时被回收, 我们在`Cargo.toml`中引入:

```rust
[dev-dependencies]
...
tempfile = "3" # 处理临时目录和临时文件
...
```

  就可以进行测试了

## 构建新的Kv Server

现在完成了SledDb和事件通知相关的实现, 我们可以尝试构建支持事件的通知, 并且使用SledDb的KV Server了, 把`examples/server.rs`拷贝出`examples/server_with_sled.rs`然后修改:

```rust
use anyhow::Result;
use async_prost::AsyncProstStream;
use futures::prelude::*;
use kv::{CommandRequest, CommandResponse, Service, ServiceInner, SledDb};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // let service = Service::new(MemTable::new());
    // let service: Service = ServiceInner::new(MemTable::new()).into();
    let service: Service<SledDb> = ServiceInner::new(SledDb::new("kv_server"))
        .fn_before_send(|res| match res.message.as_ref() {
            "" => res.message = "altered. Original message is empty".into(),
            s => res.message = format!("altered: {s}"),
        })
        .into();

    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listener on {addr}");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);

        let svc = service.clone();

        tokio::spawn(async move {
            let mut stream =
                AsyncProstStream::<_, CommandRequest, CommandResponse, _>::from(stream).for_async();

            while let Some(Ok(cmd)) = stream.next().await {
                let res = svc.execute(cmd);
                stream.send(res).await.unwrap();
            }
        });

        info!("Client {:?} disconnected", addr);
    }
}
use anyhow::Result;
use async_prost::AsyncProstStream;
use futures::prelude::*;
use kv::{CommandRequest, CommandResponse, Service, ServiceInner, SledDb};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // let service = Service::new(MemTable::new());
    // let service: Service = ServiceInner::new(MemTable::new()).into();
    let service: Service<SledDb> = ServiceInner::new(SledDb::new("kv_server"))
        .fn_before_send(|res| match res.message.as_ref() {
            "" => res.message = "altered. Original message is empty".into(),
            s => res.message = format!("altered: {s}"),
        })
        .into();

    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listener on {addr}");

    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);

        let svc = service.clone();

        tokio::spawn(async move {
            let mut stream =
                AsyncProstStream::<_, CommandRequest, CommandResponse, _>::from(stream).for_async();

            while let Some(Ok(cmd)) = stream.next().await {
                let res = svc.execute(cmd);
                stream.send(res).await.unwrap();
            }
        });

        info!("Client {:?} disconnected", addr);
    }
}
```

完成之后, 我们可以打开一个命令行, 运行: `RUST_LOG=into cargo run --example server_with_sled --quiet`, 然后在另一个命令行窗口运行:`RUST_LOG=info cargo run --example client --quiet`

此时服务器和客户端都收到彼此的请求和响应, 并且处理正常, 如果你停掉服务器, 然后再运行, 会发现客户端在尝试HSET时得到服务器旧的值, 我们的新版KV Server可以对数据进行持久化了

此外, 如果你注意看client日志, 会发现原本应该是空字符串的message包含了`altered. Original message is empty`

这是因为我们的服务注册了fn_before_send的事件通知, 对返回的数据进行了修改, 未来我们还可以用这些事件做很多事情, 比如监控数据的发送, 甚至写WAL

## 小结

今天的课程我们进一步认识到了trait的为例, 为系统设计合理的trait, 整个系统的可拓展性就大大增强了, 之后在添加新的功能的时候, 并不需要改动多少已有的代码

在使用trait做抽象的时候, 我们要衡量, 这么做的好处是什么, 它未来可以为实现者带来什么帮助, 就像我们再创写的StorageIter, 它是实现了Iterator trait, 并封装了map的处理逻辑, 让这个公共的步骤可以在Storage trait中复用

除此之外, 也进一步的熟悉了如何未带泛型参数的数据结构实现trait, 我们不仅可以为具体的数据结构实现trait, 也可以为更笼统的泛型参数实现trait

看我们写的KV Server的核心逻辑, 整体代码似乎没有太多的复杂生命周期, 或者太过于抽象的泛型结构

是的, 别看我们在介绍Rust基础知识的时候, 扎的比较深, 但是大多数写代码的时候, 并不会用到那么深的知识, Rust编译器会尽最大的努力, 让你的代码简单, 如果你用clippy这样的linter的话, 它还会进一步给你提一些建议, 让你的代码变得更简单

那么, 为什么我们还要讲的那么深入呢?

这是因为我们写代码的时候不可避免的引入第三方库, 你也看到了, 在写项目的是偶用了不少依赖, 当你使用这些库的时候, 不可避免的阅读一些它们的源码, 而这些源码, 可能有多种各种各样复杂的写法

深入的了解Rust的基础知识, 可以帮我们更快, 更清晰的阅读源码, 而更快更清晰的读懂别人的源码, 又可以更快的帮助我们用好别人的库, 从而帮助我们写好我们的代码

