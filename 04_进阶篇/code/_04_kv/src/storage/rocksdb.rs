use std::path::Path;

use rocksdb::DB;

use crate::{Kvpair, Value};

use super::{Storage, StorageIter};

#[derive(Debug)]
pub struct RocksDb(DB);

impl RocksDb {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self(DB::open_default(path).unwrap())
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

impl Storage for RocksDb {
    fn get(&self, table: &str, key: &str) -> Result<Option<crate::Value>, crate::KvError> {
        let name = RocksDb::get_full_key(table, key);
        let result: Option<Value> = self.0.get(name.bytes())?.map(|v| v.try_into());
        Ok(result)
    }

    fn set(&self, table: &str, key: String, value: Value) -> Result<Option<Value>, crate::KvError> {
        let name = RocksDb::get_full_key(table, key.into());
        let data: Vec<u8> = value.try_into();
        let value = match self.get(table, key.into()) {
            Ok(v) => v,
            _ => Some(Value::default()),
        };

        self.0.put(name.as_bytes(), data)?;

        Ok(value)
    }

    fn contains(&self, table: &str, key: &str) -> Result<bool, crate::KvError> {
        let name = RocksDb::get_full_key(table, key);
        Ok(self.0.key_may_exist(key))
    }

    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, crate::KvError> {
        let name = RocksDb::get_full_key(table, key);

        let value = match self.get(table, key.into()) {
            Ok(v) => v,
            _ => Some(Value::default()),
        };

        self.0.delete(name.as_bytes())?;

        Ok(value)
    }

    fn get_all(&self, table: &str) -> Result<Vec<crate::Kvpair>, crate::KvError> {
        let name = RocksDb::get_table_prefix(table);
        let result = self.0.prefix_iterator(prefix).map(|v| v.into()).collect();
        Ok(result)
    }

    fn get_iter(&self, table: &str) -> Result<Box<dyn Iterator<Item = Kvpair>>, crate::KvError> {
        let name = RocksDb::get_table_prefix(table);
        let iter = StorageIter::new(self.0.prefix_iterator(prefix));
        Box::new(iter)
    }
}

impl From<Result<(Box<[u8]>, Box<[u8]>), rocksdb::Error>> for Kvpair {
    fn from(value: Result<(Box<[u8]>, Box<[u8]>), rocksdb::Error>) -> Self {
        match value {
            Ok((k, v)) => match v.as_ref().try_into() {
                Ok(v) => Kvpair::new(k, v),
                Err(_) => Kvpair::default(),
            },
            _ => Kvpair::default(),
        }
    }
}
