use dashmap::{mapref::one::Ref, DashMap};

use crate::{Kvpair, Storage, Value};

/// 使用DashMap构建的MemTable. 实现了Storage trait
#[derive(Clone, Debug, Default)]
pub struct MemTable {
    tables: DashMap<String, DashMap<String, Value>>,
}

impl MemTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 如果名为name的hash table不存在则创建并返回table, 否则直接返回table
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

// 为hash table实现Storage trait
impl Storage for MemTable {
    fn get(&self, table: &str, key: &str) -> Result<Option<Value>, crate::KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.get(key).map(|v: Ref<'_, String, Value>| v.value().clone()))
    }

    fn set(&self, table: &str, key: String, value: Value) -> Result<Option<Value>, crate::KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.insert(key, value))
    }

    fn get_all(&self, table: &str) -> Result<Vec<crate::Kvpair>, crate::KvError> {
        let table = self.get_or_create_table(table);
        Ok(table
            .iter()
            .map(|v| Kvpair::new(v.key(), v.value().clone()))
            .collect())
    }

    fn contains(&self, table: &str, key: &str) -> Result<bool, crate::KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.contains_key(key))
    }

    fn del(&self, table: &str, key: &str) -> Result<Option<Value>, crate::KvError> {
        let table = self.get_or_create_table(table);
        Ok(table.remove(key).map(|(_k, v)| v))
    }

    fn get_iter(
        &self,
        table: &str,
    ) -> Result<Box<dyn Iterator<Item = crate::Kvpair>>, crate::KvError> {
        // let table = self.get_or_create_table(table);
        todo!()
    }
}
