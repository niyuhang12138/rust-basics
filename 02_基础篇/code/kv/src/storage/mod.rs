mod memory;

pub use memory::*;

use crate::{KvError, Kvpair, Value};

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

#[cfg(test)]
mod tests {
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

    // #[test]
    // fn memtable_iter_should_work() {
    //     let store = MemTable::new();
    //     test_get_iter(store);
    // }

    fn test_basic_interface(store: impl Storage) {
        // 第一次set会创建table, 插入key并返回None(因为之前没有值)
        let v = store.set("t1", "hello".into(), "world".into());
        assert!(v.unwrap().is_none());

        // 再次set同样的key会更新, 并返回之前的值
        let v1 = store.set("t1", "hello".into(), "world1".into());
        assert_eq!(v1, Ok(Some("world".into())));

        // get存在的key会得到最新的值
        let v = store.get("t1", "hello");
        assert_eq!(v, Ok(Some("world1".into())));

        // get不存在的key或者table, 会得到None
        assert_eq!(Ok(None), store.get("t1", "hello1"));
        assert!(store.get("t2", "hello1").unwrap().is_none());

        // contains存在的key返回true, 否则返回false
        assert_eq!(store.contains("t1", "hello"), Ok(true));
        assert_eq!(store.contains("t1", "hello1"), Ok(false));
        assert_eq!(store.contains("t2", "hello"), Ok(false));

        // del存在的key返回之前的值
        let v = store.del("t1", "hello");
        assert_eq!(v, Ok(Some("world1".into())));

        // del不存在的key或者table返回None
        assert_eq!(Ok(None), store.del("t1", "hello1"));
        assert_eq!(Ok(None), store.del("t2", "hello"))
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

    // fn test_get_iter(store: impl Storage) {
    //     store.set("t2", "k1".into(), "v1".into()).unwrap();
    //     store.set("t2", "k2".into(), "v2".into()).unwrap();
    //     let mut data = store.get_iter("t2").unwrap().collect::<Vec<_>>();

    //     data.sort_by(|a, b| a.partial_cmp(b).unwrap());
    //     assert_eq!(
    //         data,
    //         vec![
    //             Kvpair::new("k1", "v1".into()),
    //             Kvpair::new("k2", "v2".into())
    //         ]
    //     )
    // }
}
