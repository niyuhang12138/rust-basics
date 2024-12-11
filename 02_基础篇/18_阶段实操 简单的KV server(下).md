# 阶段实操: 简单的KV server(下)

上篇我们的KV store刚开了头, 写好了基本的接口, 你是不是摩拳擦掌准备开始写具体的实现代码了? 别着急, 当定义好接口后, 先不忙实现, 在撰写更多代码前, 我们可以从一个使用者的角度来体验接口如何使用, 是否好用, 反观设计有哪些地方有待完善

还是按照上一讲的顺序来一个个测试: 首先来构建我们的协议层

## 实现并验证协议层

先创建一个库项目, 在`Cargo.toml`中添加依赖

```rust
[package]
name = "kv_server"
version = "0.1.0"
edition = "2021"

[dependencies]
bytes = "1.9.0"
prost = "0.13.4"
tracing = "0.1.41"

[build-dependencies]
prost-build = "0.13.4"

[dev-dependencies]
anyhow = "1.0.94"
async-prost = "0.4.0"
futures = "0.3.31"
tokio = { version = "1.42.0", features = ["rt", "rt-multi-thread", "io-util", "macros", "net"] }
tracing-subscriber = "0.3.19"
```

然后在项目根目录下创建`abi.proto`, 把上文中的protobuf的代码放进去, 在根目录下创建`build.rs`;

```rust
fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(&["."]);
    config.type_attribute(".", "#[derive(PartialOrd)]");
    config
    .out_dir("src/pb")
    .compile_protos(&["abi.proto"], &["."])
    .unwrap();
}
```

这里我们为编译出来的代码添加一些属性, 比如为protobuf的bytes类型生成的Bytes而非缺省的`Vec<u8>`, 为所有理性加入PartialOrd派生宏, 关于prost-build的拓展, 你可以看文档

记得创建src/pb目录, 否则编译不通过, 现在, 在项目根目录下cargo build会生成`src/pb/abi.rs`文件, 里面包含所有protobuf定义消息的Rust数据类型, 我们创建`src/pb/mod.rs`, 引入`abi.rs`, 并做一些基本的类型转换:

```rust
pub mod abi;

use abi::{command_request::RequestData, *};

impl CommandRequest {
    // 创建HSET命令
    pub fn new_hset(table: impl Into<String>, key: impl Into<String>, value: Value) -> Self {
        Self {
            request_data: Some(RequestData::Hset(Hset {
                table: key.into(),
                pair: Some(value),
            })),
        }
    }
}

impl Kvpair {
  // 创建一个新的kv pair
  pub fn new(key: impl Into<String>, value: Value) -> Self {
    Self {
      key: key.into(),
      value: Some(value)
    }
  }
}

/// 从String转换为Value
impl From<String> for Value {
  fn from(value: String) -> Self {
      Self {
        value: Some(value::Value::String(value))
      }
  }
}

/// 从&str转换成Value
impl From<&str> for Value {
  fn from(value: &str) -> Self {
      Self {
        value: Some(value::Value::String(s))
      }
  }
}
```

最后, 在`src/lib.rs`中引入pb模块

```rust
mod pb;
pub use pn::abi::*;
```

这样我们就有了能把KV Server最监本的protobuf解耦运转起来的代码

在根目录下创建examples, 这样可以写一些代码测试客户端和服务器之间的协议, 我们可以先创建一个`examples/client.rs`文件, 写入如下代码:

```rust
use anyhow::Result;
use async_prost::AsyncProstStream;
use futures::prelude::*;
use kv_server::{CommandRequest, CommandResponse};
use tokio::net::TcpStream;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let addr = "127.0.0.1:9527";

    // 连接服务器
    let stream = TcpStream::connect(addr).await?;

    // 使用AsyncProstStream处理Tcp Frame
    let mut client =
        AsyncProstStream::<_, CommandResponse, CommandRequest, _>::from(stream).for_async();

    // 生成一个HSET命令
    let cmd = CommandRequest::new_hset("table1", "hello", "world".to_string().into());

    // 发送HSET命令
    client.send(cmd).await?;
    // if let Some(Ok(data)) = client.next
    if let Some(Ok(data)) = client.next().await {
        info!("Got response {:?}", data);
    }

    Ok(())
}
```

这段代码连接服务器的9527接口, 发送一个HSET命令出去, 然后等待服务器的响应

同样的, 我们创建一个`examples/dummy_server.rs`文件, 写入文件

```rust
use anyhow::Result;
use async_prost::AsyncProstStream;
use futures::prelude::*;
use kv_server::{CommandRequest, CommandResponse};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {addr}");
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);
        tokio::spawn(async move {
            let mut stream =
                AsyncProstStream::<_, CommandRequest, CommandResponse, _>::from(stream).for_async();
            while let Some(Ok(msg)) = stream.next().await {
                info!("Got a new command: {:?}", msg);
                let mut resp = CommandResponse::default();
                resp.status = 404;
                resp.message = "Not found".to_string();
                stream.send(resp).await.unwrap();
            }
            info!("Client {:?} disconnected", addr);
        });
    }
}
```

在这段代码中, 服务器监听9527端口, 对任何客户端的请求, 一律返回status = 404, message是"Not found"的响应

如果ui这两段代码的异步和网络处理半懂不懂, 没关系, 后面会讲到

我们可以打开一个命令行窗口, 运行:`RUST_LOG=into cargo run --example dummy_server --quiet`, 然后在另一个命令行窗口运行: `RUST_LOG=into cargo run --example client --quiet`

此时服务器和客户端都收到彼此的请求和响应, 协议层看上去运作良好, 一旦验证通过, 就可以进入下一步, 因为协议层的其他代码都只是工作量而已, 在之后需要的时候可以慢慢实现

## 实现并验证Storage trait

接下来构建Storage trait

我们上一讲谈到了如何使用嵌套的支持并发的im-memory HashMap来实现storage trait, 由于`Arc<RwLock<HashMap<K, V>>>`这样的支持并发的HashMap是一个刚需, Rust生态有很多先关的crate支持, 这里我们可以使用dashmap创建一个MemTable结构, 来实现Storage trait

先创建src/storage目录, 然后创建src/storage/mod.rs, 把刚才讨论的trait代码fnagjinqu后, 在`src/lib.rs`中引入"mod storage", 此时会发现一个错误: 并定义KvError

所以来定义KvError, 我们之前讨论了如果使用thiserror的派生宏来定义错误类型, 今天就用它来定义KvError, 创建`src/error.rs`然后填入:

```rust
use crate::Value;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum KvError {
    #[error("Not found for table: {0}, key: {1}")]
    NotFound(String, String),

    #[error("Cannot parse command: `{0}`")]
    InvalidCommand(String),

    #[error("Cannot convert value {:0} to {1}")]
    ConvertError(Value, &'static str),

    #[error("Cannot process command {0} with table: {1}, key: {2}, Error: {}")]
    StorageError(&'static str, String, String, String),

    #[error("Failed to encode protobuf message")]
    EncodeError(#[from] prost::EncodeError),

    #[error("Failed to decode protobuf message")]
    DecodeError(#[from] prost::DecodeError),

    #[error("Internal error: {0}")]
    Internal(String),
}
```

这些error的定义其实是在实现过程中逐步添加的, 但为了讲解方便, 先一次性添加, 对于Storage的实现, 我们只关心StorageError, 其他的error定义未来会用到

同样, 在`src/lib.rs`下引入mod error

`src/storage/mod.rs`是这个样子的:

```rust
use crate::{KvError, Kvpair, Value};

/// 对存储的出现, 我们不关心数据存在呢, 但需要定义外界如何和存储打交道
pub trait Storage {
    // 从一个HashTable里获取一个key的value
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    // 从一个HashTable里设置一个key的value, 返回旧的value
    fn set(&self, table: &str, value: Value) -> Result<Option<Value>, KvError>;

    // 查看HashTable中是否有key
    fn contains(&self, table: &str, key: &str) -> Result<bool, KvError>;

    // 从HashTable中删除一个key
    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    // 遍历HashTable, 返回所有的kv pair(这个接口不好)
    fn get_all(&self, table: &str) -> Result<Vec<Kvpair>, KvError>;

    // 遍历HashTable, 返回kv pair的Iterator
    fn get_iter(&self, table: &str) -> Result<Box<dyn Iterator<Item = Kvpair>>, KvError>;
}
```

代码目前没有编译错误, 可以在这个文件末尾添加测试文件, 尝试使用这些接口, 当然, 我们还没有MemTable, 但通过Storage trait已经大概知道MemTable怎么用, 所以可以先写一段测试体验一下:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memtable_basic_interface_should_work() {
        let store = MemTable::new();
        test_basi_interface(store);
    }

    #[test]
    fn memtable_get_all_should_work() {
        let store = MemTable::new();
        test_get_all(store);
    }

    fn test_basi_interface(store: impl Storage) {
        // 第一次 set 会创建 table，插入 key 并返回 None（之前没值）
        let v = store.set("t1", "hello".into(), "world".into());
        assert!(v.unwrap().is_none());
        // 再次 set 同样的 key 会更新，并返回之前的值
        let v1 = store.set("t1", "hello".into(), "world1".into());
        assert_eq!(v1, Ok(Some("world".into())));

        // get 存在的 key 会得到最新的值
        let v = store.get("t1", "hello");
        assert_eq!(v, Ok(Some("world1".into())));

        // get 不存在的 key 或者 table 会得到 None
        assert_eq!(Ok(None), store.get("t1", "hello1"));
        assert!(store.get("t2", "hello1").unwrap().is_none());

        // contains 纯在的 key 返回 true，否则 false
        assert_eq!(store.contains("t1", "hello"), Ok(true));
        assert_eq!(store.contains("t1", "hello1"), Ok(false));
        assert_eq!(store.contains("t2", "hello"), Ok(false));

        // del 存在的 key 返回之前的值
        let v = store.del("t1", "hello");
        assert_eq!(v, Ok(Some("world1".into())));

        // del 不存在的 key 或 table 返回 None
        assert_eq!(Ok(None), store.del("t1", "hello1"));
        assert_eq!(Ok(None), store.del("t2", "hello"));
    }

    fn test_get_all(store: impl Storage) {
        store.set("t2", "k1".into(), "v1".into()).unwrap();
        store.set("t2", "k2".into(), "v2".into()).unwrap();
        let mut data = store.get_all("t2").unwrap();
        data.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(
            data,
            vec![
                Kvpair::new("k1", "v1".into()),
                Kvpair::new("k2", "v2".into())
            ]
        )
    }
}
```

这猴子那个在写实现之前写单元测试, 是标准的TDD(Test-Driven Development)方式

构建完trait之后, 为这个trait撰写测试代码, 因为写测试代码是个很好的验证接口是否好用的时机, 毕竟我们不希望实现trait之后, 才发现trait定义的有瑕疵, 需要修改, 这个是否改动的代价就比较大了

所以, 当trait敲定完毕, 就可以开始写trait的测试代码了, 在使用过程中仔细感受, 如果写测试用例是用的不舒服, 或者为了使用它需要做很多繁琐的操作, 那么可以重新审视trait的设计

你如果仔细看单元测试的代码, 就会发现我始终秉持测试trait接口的思想, 尽管在测试中需要一个实际的数据结构进行trait方法的测试, 但核心的测试代码都用的泛型函数, 让这些代码只跟trait相关

这样, 依赖可以避免某个具体trait实现的干扰, 而来之后想加入更多trait实现时, 可以共享测试代码, 比如未来想支持DiskTable, 那么只需要加几个测试用例, 调用已有的泛型函数即可

搞定测试, 确认trait设计没有什么问题之后, 我们来写具体的实现, 可以创建`src/storage/memory.rs`来构建MemTable:

```rust
use crate::{KvError, Kvpair, Storage, Value};
use dashmap::{mapref::one::Ref, DashMap};

#[derive(Debug, Clone, Default)]
pub struct MemTable {
    tables: DashMap<String, DashMap<String, Value>>,
}

impl MemTable {
    /// 创建一个缺省的MemTable
    pub fn new() -> Self {
        Self::default()
    }

    // 如果名为name的hash table不存在, 则创建, 否则返回
    fn get_or_create_table(&self, name: &str) -> Ref<String, DashMap<String, Value>> {
        match self.tables.get(name) {
            Some(table) => table,
            None => {
                let entry = self.tables.entry(name.into()).or_default();
                entry.downgrade()
            }
        }
    }
}

impl Storage for MemTable {
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.get(key).map(|v| v.value().clone()))
    }

    fn set(&self, table: &str, key: String, value: Value) -> Result<Option<Value>, KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.insert(key, value))
    }

    fn contains(&self, table: &str, key: &str) -> Result<bool, KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.contains_key(key))
    }

    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.remove(key).map(|(_k, v)| v))
    }

    fn get_all(&self, table: &str) -> Result<Vec<Kvpair>, KvError> {
        let table = self.get_or_create_table(table);
        Ok(table
            .iter()
            .map(|v| Kvpair::new(v.key(), v.value().clone()))
            .collect())
    }

    fn get_iter(&self, table: &str) -> Result<Box<dyn Iterator<Item = Kvpair>>, KvError> {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_or_create_table_should_work() {
        let store = MemTable::new();
        assert!(!store.tables.contains_key("t1"));
        store.get_or_create_table("t1");
        assert!(store.tables.contains_key("t1"));
    }
}
```

除了get_iter外, 这个实现代码非常简单, 相信你看一下dashmap的文档, 也能很快的写出来, get_iter写起来稍微有些难度, 我们稍等再说

实现完成之后, 我们可以测试它是否符合预期, 注意现在`src/storage/memory.rs`还没有被添加, 所以cargo并不会编译它, 要在`src/storage/mod.rs`开头添加代码

```rust
mod momory;
pub use memory::MemTable;
```

## 实现并验证CommandService trait

Storage trait我们就算基本验证通过了, 现在再来验证CommandService

我们创建`src/service`目录, 以及创建`src/service/mod.rs`和`src/service/command_service.rs`文件, 并在`src/service/mod.rs`写入:

```rust
mod command_service;

use crate::{CommandResponse, Storage};
pub trait CommandService {
    fn execute(self, store: impl Storage) -> CommandResponse;
}
```

不要忘记在src/lib.rs中加入service

然后在`src/service/command_service.rs`中, 我们可以先写一些测试, 为了简单起见, 就列HSET, HGET. HGETALL三个命令:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_request::RequestData;

    #[test]
    fn hset_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("t1", "hello", "world".into());
        let res = dispatch(cmd.clone(), &store);
        assert_res_ok(res, &[10.into()], &[]);
    }

    #[test]
    fn hget_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("score", "u1", 10.into());
        dispatch(cmd, &store);
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_ok(res, &[10.into()], &[]);
    }
    #[test]
    fn hget_with_non_exist_key_should_return_404() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_error(res, 404, "Not found");
    }

    fn dispatch(cmd: CommandRequest, store: &impl Storage) -> CommandResponse {
        match cmd.request_data.unwrap() {
            RequestData::Hset(v) => v.execute(store),
            _ => todo!(),
        }
    }

    // 测试成功返回的结果
    fn assert_res_ok(mut res: CommandResponse, values: &[Value], pairs: &[Kvpair]) {
        res.pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(res.status, 200);
        assert_eq!(res.message, "");
        assert_eq!(res.values, values);
        assert_eq!(res.pairs, pairs);
    }

    // 测试失败返回的结果
    fn assert_res_error(res: CommandResponse, code: u32, msg: &str) {
        assert_eq!(res.status, code);
        assert!(res.message.contains(msg));
        assert_eq!(res.values, &[]);
        assert_eq!(res.pairs, &[]);
    }
}
```

这些测试的作用就是验证产品需求, 比如:

- HSET成功返回上一次的值(这和Redis略有不同, Redis返回表示多少受影响的一个整数)
- HGET返回Value
- HGETALL返回一组无需的Kvpair

目前这些测试是无法通过的, 因为里面使用了一些未定义的方法, 比如`10.into()`: 想把整数10转换成一个Value, `CommandRequest::new_hgetall("score")`: 想生成一个HGETALL命令

为什么要这么写? 因为如果是CommandService接口的使用者, 自然希望使用这个接口的时候, 地哦啊用感觉非常简单明了

如果接口期待一个Value, 但在上下文中拿到的是10, "hello"这样的值, 那我们作为设计者就要考虑为Value实现`From<T>`. 这样调用的时候最方便, 同样的, 对于生成CommandRequest这个数据结构, 也可以添加一些辅助函数, 来让调用者更清晰

到现在为止我们已经写了两轮测试了, 相信你对测试代码的作用有大概的理解, 我们来总结一下:

1. 验证并帮助接口迭代
2. 验证产品需求
3. 通过使用核心逻辑, 帮助我们更好的思考外围逻辑并反推其实现

前两点式基本的, 也是很多人对TDD的理解, 其实还有更重要的是第三点, 除了前面的辅助函数之外, 我们在测试代码中, 还看到了dispatch函数, 它目前用来辅助测试, 但紧接着你会发现, 这样的辅助函数, 可以合并到核心代码中, 这才是测试驱动开发的实质

根据测试, 我们需要再`src/pb/mod.rs`中添加相关的外围逻辑, 首先是CommandRequest的一些方法, 之前写了new_hset, 现在再加入new_hget和new_hgetall:

```rust
impl CommandRequest {
    // 创建HSET命令
    pub fn new_hset(table: impl Into<String>, key: impl Into<String>, value: Value) -> Self {
        Self {
            request_data: Some(RequestData::Hset(Hset {
                table: table.into(),
                pair: Some(Kvpair::new(key, value)),
            })),
        }
    }

    // 创建HGET命令
    pub fn new_hget(table: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            request_data: Some(RequestData::Hget(Hget {
                table: table.into(),
                key: key.into(),
            })),
        }
    }

    // 创建HGETALL命令
    pub fn new_hgetall(table: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            request_data: Some(RequestData::Hgetall(Hgetall {
                table: table.into(),
            })),
        }
    }
}
```

然后对value的`From<i64>`的实现:

```rust
/// 从 i64转换成 Value
impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self {
            value: Some(value::Value::Integer(i)),
        }
    }
}
```

测试代码目前就可以编译通过了, 然后测试显然会失败, 因为还没有做具体的实现, 我们在`src/service/command_service.rs`下添加trait的实现代码:

```rust
impl CommandService for Hget {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.get(&self.table, &self.key) {
            Ok(Some(v)) => v.into(),
            Ok(None) => KvError::NotFound(self.table, self.key).into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hgetall {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.get_all(&self.table) {
            Ok(v) => v.into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hset {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match self.pair {
            Some(v) => match store.set(&self.table, v.key, v.value.unwrap_or_default()) {
                Ok(Some(v)) => v.into(),
                Ok(None) => Value::default().into(),
                Err(e) => e.into(),
            },
            None => Value::default().into(),
        }
    }
}
```

这自然会引发更多的编译错误, 因为我们很多地方都是用了into方法, 却没有实现相应的转化, 比如Value到CommandResponse的转换, KvError到CommandResponse的转换, `Vec<Kvpair>`到CommandResponse的转换等等

所以在`src/pb/mod.rs`里继续补上相应的外围逻辑:

```rust
/// 从Value转换成CommandResponse
impl From<Value> for CommandResponse {
    fn from(value: Value) -> Self {
        Self {
            status: StatusCode::OK.as_u16() as _,
            values: vec![value],
            ..Default::default()
        }
    }
}

/// 从Vec<KvPair>转换成CommandResponse
impl From<Vec<Kvpair>> for CommandResponse {
    fn from(value: Vec<Kvpair>) -> Self {
        Self {
            status: StatusCode::OK.as_u16() as _,
            pairs: value,
            ..Default::default()
        }
    }
}

/// 从KvError转换成CommandResponse
impl From<KvError> for CommandResponse {
    fn from(value: KvError) -> Self {
        let mut result = Self {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
            message: value.to_string(),
            values: vec![],
            pairs: vec![],
        };

        match value {
            KvError::NotFound(_, _) => result.status = StatusCode::NOT_FOUND.as_u16() as _,
            KvError::InvalidCommand(_) => result.status = StatusCode::BAD_REQUEST.as_u16() as _,
            _ => {}
        };

        result
    }
}
```

从前面写接口到这里具体实现, 不知道你是否感受到了这样的一种模式: 在Rust下, 但凡出现两个数据结构的转换, 你都可以先以表示出来, 之后再去补`From<T>`的实现, 如果相互转换的数据都不是你定义的数据结构, 那么你需要把其中之一用struct包裹一下, 来绕过之前提到的孤儿规则

## 最后拼图: Service结构的实现

所有的接口, 包括客户端 / 服务器的协议接口, Storage trait和CommandService trait都验证好了, 接下来就是考虑如何用一个数据结构把所有的这些东西串联起来

依旧从使用者的角度来看如何调用它, 为此, 我们在`src/service/mod.rs`里添加如下的测试代码:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemTable, Value};
    use std::thread;

    #[test]
    fn service_should_works() {
        // 我们需要一个 service 结构至少包含 Storage
        let service = Service::new(MemTable::default());

        // service 可以运行在多线程环境下，它的 clone 应该是轻量级的
        let cloned = service.clone();

        // 创建一个线程，在 table t1 中写入 k1, v1
        let handle = thread::spawn(move || {
            let res = cloned.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
            assert_res_ok(res, &[Value::default()], &[]);
        });
        handle.join().unwrap();

        // 在当前线程下读取 table t1 的 k1，应该返回 v1
        let res = service.execute(CommandRequest::new_hget("t1", "k1"));
        assert_res_ok(res, &["v1".into()], &[]);
    }
}

#[cfg(test)]
use crate::{Kvpair, Value};

// 测试成功返回的结果
#[cfg(test)]
pub fn assert_res_ok(mut res: CommandResponse, values: &[Value], pairs: &[Kvpair]) {
    res.pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(res.status, 200);
    assert_eq!(res.message, "");
    assert_eq!(res.values, values);
    assert_eq!(res.pairs, pairs);
}

// 测试失败返回的结果
#[cfg(test)]
pub fn assert_res_error(res: CommandResponse, code: u32, msg: &str) {
    assert_eq!(res.status, code);
    assert!(res.message.contains(msg));
    assert_eq!(res.values, &[]);
    assert_eq!(res.pairs, &[]);
}
```

注意这里的assert_res_ok和assert_res_error是从`src/service/command_service.rs`中挪过来的, 在开发过程中, 不光产品代码需要不断重构, 测试代码也需要重构来贯彻DRY思想

我们见过很多生产环境下的代码, 产品功能部分还说得过去, 但测试代码就很垃圾, 这样非常不好

测试代码的质量也要和产品代码的质量同等等级, 好的开发者写的测试代码可读性也是非常强的, 你可以对比上面写的三段测试代码多多感受

在撰写测试代码的时候, 我们要特别注意: 测试代码要围绕着系统稳定的部分, 也就是接口, 来测试, 而尽可能的少测试实现

因为产品代码和测试代码, 两者总需要一个相对稳定的, 既然产品代码会不断的根据需求变动, 测试代码就必须要稳定一些

那什么样的测试代码算是稳定呢? 测试接口的代码是稳定的, 只要接口不变, 无论具体实现如何变化, 哪怕今天引入了一个新的算法, 明天重写实现, 测试代码依旧能够凛然不动

在这段测试代码中, 已经敲定了Service这个数据结构的使用蓝图, 它可以跨线程, 可以调用execute来执行某个CommandRequest命令, 返回CommandResponse

根据这些想法, 我们在`src/service/mod.rs`里添加Service的声明和实现:

```rust
/// Service数据结构
pub struct Service<Store = MemTable> {
    inner: Arc<ServiceInner<Store>>,
}

/// Service内部数据结构
pub struct ServiceInner<Store> {
    store: Store,
}

impl<Store: Storage> Service<Store> {
    pub fn new(store: Store) -> Self {
        Self {
            inner: Arc::new(ServiceInner { store }),
        }
    }

    pub fn execute(&self, cmd: CommandRequest) -> CommandResponse {
        debug!("Got request: {:?}", cmd);
        // TODO: 发送on_receiver
        let res = dispatch(cmd, &self.inner.store);
        debug!("Executed response: {:?}", res);
        // TODO: 发送on_executed事件

        res
    }
}

// Request中得到Response, 目前处理HGET / HGETALL / HSET
pub fn dispatch(cmd: CommandRequest, store: &impl Storage) -> CommandResponse {
    match cmd.request_data {
        Some(RequestData::Hget(param)) => param.execute(store),
        Some(RequestData::Hgetall(param)) => param.execute(store),
        Some(RequestData::Hset(param)) => param.execute(store),
        None => KvError::InvalidCommand("Request has no data".into()).into(),
        _ => KvError::Internal("Not implemented".into()).into(),
    }
}
```

这段代码有地方值得注意:

1. 首先Service结构内部有一个ServiceInner存放实际的数据结构, Service只是用Arc包裹了ServiceInner, 这也是Rust下的一个历, 把需要再多线程下clone的主体和其内部结构分开, 这样代码逻辑更加清晰
2. execute方法目前就是调用了dispatch, 但它未来潜在可以做一些事件分发, 这样处理体现了SRP原则
3. dispatch其实就是把测试代码的dispatch逻辑移动过来改动了一下

再一次, 我们重构了测试代码, 把它的辅助函数变成了产品代码的一部分

## 新的server

现在的处理逻辑都已经完成了, 可以写个新的example测试服务器代码

把之前的`examples/dummy_server.rs`复制一份, 成为`examples/server.rs`, 然后引入Service

```rust
use anyhow::Result;
use async_prost::AsyncProstStream;
use futures::prelude::*;
use kv_server::{CommandRequest, CommandResponse, MemTable, Service};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    // 初始化servicer
    let service = Service::new(MemTable::new());

    let addr = "127.0.0.1:9527";
    let listener = TcpListener::bind(addr).await?;
    info!("Start listening on {addr}");
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Client {:?} connected", addr);
        // 复制一份service
        let svc = service.clone();
        tokio::spawn(async move {
            let mut stream =
                AsyncProstStream::<_, CommandRequest, CommandResponse, _>::from(stream).for_async();
            while let Some(Ok(cmd)) = stream.next().await {
                let res = svc.execute(cmd);
                stream.send(res).await.unwrap();
            }
            info!("Client {:?} disconnected", addr);
        });
    }
}
```

完成之后, 打开一个命令行窗口, 运行: `RUST_LOG=info cargo run --example server --quiet`, 然后在另外一个命令行窗口运行: `RUST_LO=info cargo run --example client --quiet`, 此时服务器和客户端都收到了彼此的请求和响应, 并且处理正常

我们的KV server第一版的基本功能就完工了, 当然目前还只能处理三个命令, 剩下六个需要你自己完成

## 小结

KV Server并不是一个很难的项目, 但想要把它写好, 并不简单, 如果你跟着讲解一步步走下来, 可能感受到一个有潜在生产环境质量的Rust项目应该如何开发, 在这上下两讲内容中, 有两点我们一定要认真领会

第一点: 你要对需求有一个清晰的把握, 找出其中不稳定的部分和比较稳定的部分, 在KV server中, 不稳定的部分是对各种新的命令的支持, 以及对不同storage的支持, 所以需要偶见接口来消除不稳定的因素, 让不稳定的部分可以用一种稳定的方式来管理

第二点, 代码和测试可以围绕着接口螺旋前进, 使用TDD可以帮助我们进行这种螺旋式的迭代, 在一个设计良好的系统中, 接口是稳定的, 测试接口的代码是稳定的, 实现可以是不稳定的, 在迭代开发的过程中, 我们要不断的重构, 让测试代码和产品代码都往最优的方向发展

纵观我们写的KV Server, 包括测试在内, 你很难发现有函数或者方法超过50行, 代码可读性很强, 几乎不需要注释, 就可以理解, 另外因为都是用接口做的交互, 未来维护和添加新的功能, 也基本上满足了OCP原则, 除了dispatch函数需要很小的修改外, 其他新的代码都是在实现一些接口而已

相信你能初步感受Rust下撰写代码的最佳实践, 如果你之前用其他语言开发, 已经采用了类似的最佳实践, 那么可以感受一下同样的实践在Rust下使用的那种优雅; 
