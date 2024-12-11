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

