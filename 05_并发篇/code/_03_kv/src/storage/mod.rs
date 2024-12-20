use crate::{KvError, Kvpair, Value};

mod memory;
// mod rocksdb;
mod sleddb;

pub use memory::*;
// pub use rocksdb::*;
pub use sleddb::*;

/// 对存储的抽象. 我们不关心数据存在哪里, 但需要定义外界如何和存储打交道
pub trait Storage {
    /// 从HashTable里获取一个key的value
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    /// 从一个HashTable中设置一个key的value, 返回旧的value
    fn set(&self, table: &str, key: String, value: Value) -> Result<Option<Value>, KvError>;

    /// 查看HashTable中是否存在key
    fn contains(&self, table: &str, key: &str) -> Result<bool, KvError>;

    /// 从HashTable中删除一个key
    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, KvError>;

    /// 遍历HashTable, 返回所有的kv pair
    fn get_all(&self, table: &str) -> Result<Vec<Kvpair>, KvError>;

    /// 遍历HashTable, 返回Kvpair的Iterator
    fn get_iter(&self, table: &str) -> Result<Box<dyn Iterator<Item = Kvpair>>, KvError>;
}

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

#[cfg(test)]
mod tests {

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn memtable_basic_interface_should_work() {
        let store = MemTable::new();
        test_basic_interface(store);
    }

    #[test]
    fn memetable_get_all_should_work() {
        let store = MemTable::new();
        test_get_all(store);
    }

    #[test]
    fn memtable_iter_should_work() {
        let store = MemTable::new();
        test_get_iter(store);
    }

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

    // #[test]
    // fn rocksdb_basic_interface_should_work() {
    //     let dir = tempdir().unwrap();
    //     let store = RocksDb::new(dir);
    //     test_get_all(store);
    // }

    // #[test]
    // fn rocks_iter_should_work() {
    //     let dir = tempdir().unwrap();
    //     let store = RocksDb::new(dir);
    //     test_get_iter(store);
    // }

    fn test_basic_interface(store: impl Storage) {
        // 第一次 set 会创建 table，插入 key 并返回 None（之前没值）
        let v = store.set("t1", "hello".into(), "world".into());
        assert!(v.unwrap().is_none());
        // 再次 set 同样的 key 会更新，并返回之前的值
        let v1 = store.set("t1", "hello".into(), "world1".into());
        assert_eq!(v1.unwrap(), Some("world".into()));

        // get 存在的 key 会得到最新的值
        let v = store.get("t1", "hello");
        assert_eq!(v.unwrap(), Some("world1".into()));

        // get 不存在的 key 或者 table 会得到 None
        assert_eq!(None, store.get("t1", "hello1").unwrap());
        assert!(store.get("t2", "hello1").unwrap().is_none());

        // contains 纯在的 key 返回 true，否则 false
        assert!(store.contains("t1", "hello").unwrap());
        assert!(!store.contains("t1", "hello1").unwrap());
        assert!(!store.contains("t2", "hello").unwrap());

        // del 存在的 key 返回之前的值
        let v = store.del("t1", "hello");
        assert_eq!(v.unwrap(), Some("world1".into()));

        // del 不存在的 key 或 table 返回 None
        assert_eq!(None, store.del("t1", "hello1").unwrap());
        assert_eq!(None, store.del("t2", "hello").unwrap());
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

    fn test_get_iter(store: impl Storage) {
        store.set("t2", "k1".into(), "v1".into()).unwrap();
        store.set("t2", "k2".into(), "v2".into()).unwrap();
        let mut data = store.get_iter("t2").unwrap().collect::<Vec<_>>();

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
