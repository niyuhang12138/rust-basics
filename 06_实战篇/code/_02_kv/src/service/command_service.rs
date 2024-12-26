use crate::*;

impl CommandService for Hget {
    fn execute(self, store: &impl super::Storage) -> super::CommandResponse {
        match store.get(&self.table, &self.key) {
            Ok(Some(v)) => v.into(),
            Ok(None) => KvError::NotFound(format!("table {}, key {}", self.table, self.key)).into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hmget {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        self.keys
            .iter()
            .map(|key| match store.get(&self.table, key) {
                Ok(Some(v)) => v.into(),
                _ => Value::default(),
            })
            .collect::<Vec<_>>()
            .into()
    }
}

impl CommandService for Hgetall {
    fn execute(self, store: &impl super::Storage) -> super::CommandResponse {
        match store.get_all(&self.table) {
            Ok(v) => v.into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hset {
    fn execute(self, store: &impl super::Storage) -> super::CommandResponse {
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

impl CommandService for Hmset {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        self.pairs
            .into_iter()
            .map(
                |pair| match store.set(&self.table, pair.key, pair.value.unwrap_or_default()) {
                    Ok(Some(v)) => v.into(),
                    _ => Value::default(),
                },
            )
            .collect::<Vec<_>>()
            .into()
    }
}

impl CommandService for Hdel {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.del(&self.table, &self.key) {
            Ok(Some(v)) => v.into(),
            Ok(None) => KvError::NotFound(format!("table {}, key {}", self.table, self.key)).into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hmdel {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        self.keys
            .iter()
            .map(|key| match store.del(&self.table, key) {
                Ok(Some(v)) => v.into(),
                _ => Value::default(),
            })
            .collect::<Vec<_>>()
            .into()
    }
}

impl CommandService for Hexist {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        match store.contains(&self.table, &self.key) {
            Ok(v) => Value::from(v).into(),
            Err(e) => e.into(),
        }
    }
}

impl CommandService for Hmexist {
    fn execute(self, store: &impl Storage) -> CommandResponse {
        self.keys
            .iter()
            .map(|key| match store.contains(&self.table, key) {
                Ok(v) => v.into(),
                _ => Value::default(),
            })
            .collect::<Vec<_>>()
            .into()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn hget_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("score", "u1", 10.into());
        dispatch(cmd, &store);
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_ok(&res, &[10.into()], &[]);
    }

    #[test]
    fn hget_non_exist_key_should_return_404() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hget("score", "u1");
        let res = dispatch(cmd, &store);
        assert_res_error(&res, 404, "Not found");
    }

    #[test]
    fn hmget_should_work() {
        let store = MemTable::new();
        set_pair_to_store("hm", vec![("k1", "v1"), ("k2", "v2"), ("k3", "v3")], &store);
        let cmd = CommandRequest::new_hmget("hm", vec!["k1".into(), "k2".into(), "k3".into()]);
        let res = dispatch(cmd, &store);
        assert_res_ok(&res, &["v1".into(), "v2".into(), "v3".into()], &[]);
    }

    #[test]
    fn hmget_not_key_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hmget("hm", vec!["k1".into(), "k2".into(), "k3".into()]);
        let res = dispatch(cmd, &store);
        assert_res_ok(
            &res,
            &[Value::default(), Value::default(), Value::default()],
            &[],
        );
    }

    #[test]
    fn hgetall_should_work() {
        let store = MemTable::new();

        set_pair_to_store(
            "score",
            vec![("u1", 10), ("u2", 8), ("u3", 11), ("u1", 6)],
            &store,
        );

        let cmd = CommandRequest::new_hgetall("score");
        let res = dispatch(cmd, &store);
        let pairs = &[
            Kvpair::new("u1", 6.into()),
            Kvpair::new("u2", 8.into()),
            Kvpair::new("u3", 11.into()),
        ];
        assert_res_ok(&res, &[], pairs);
    }

    #[test]
    fn hset_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("t1", "hello", "world".into());
        let res = dispatch(cmd.clone(), &store);
        assert_res_ok(&res, &[Value::default().into()], &[]);
        let res = dispatch(cmd, &store);
        assert_res_ok(&res, &["world".into()], &[]);
    }

    #[test]
    fn hmset_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hmset(
            "hm",
            vec![
                Kvpair::new("k1", "v1".into()),
                Kvpair::new("k2", "v2".into()),
                Kvpair::new("k3", "v3".into()),
            ],
        );
        let res = dispatch(cmd.clone(), &store);
        assert_res_ok(
            &res,
            &[Value::default(), Value::default(), Value::default()],
            &[],
        );

        let res = dispatch(cmd, &store);
        assert_res_ok(&res, &["v1".into(), "v2".into(), "v3".into()], &[]);
    }

    #[test]
    fn hdel_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("d1", "d1", "hello".into());
        dispatch(cmd, &store);
        let cmd = CommandRequest::new_hdel("d1", "d1");
        let res = dispatch(cmd, &store);
        assert_res_ok(&res, &["hello".into()], &[]);
    }

    #[test]
    fn hdel_not_key_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hdel("d1", "d1");
        let res = dispatch(cmd, &store);
        assert_res_error(&res, 404, "Not found");
    }

    #[test]
    fn hmdel_should_work() {
        let store = MemTable::new();

        set_pair_to_store("hm", vec![("k1", "v1"), ("k2", "v2"), ("k3", "v3")], &store);

        let cmd = CommandRequest::new_hmdel("hm", vec!["k1".into(), "k2".into(), "k3".into()]);
        let res = dispatch(cmd.clone(), &store);
        assert_res_ok(&res, &["v1".into(), "v2".into(), "v3".into()], &[]);
        let res = dispatch(cmd, &store);
        assert_res_ok(
            &res,
            &[Value::default(), Value::default(), Value::default()],
            &[],
        );
    }

    #[test]
    fn hexist_key_exist_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hset("e1", "e1", "hello".into());
        dispatch(cmd, &store);
        let cmd = CommandRequest::new_hexist("e1", "e1");
        let res = dispatch(cmd, &store);
        assert_res_ok(&res, &[true.into()], &[]);
    }

    #[test]
    fn hexist_key_not_exist_should_work() {
        let store = MemTable::new();
        let cmd = CommandRequest::new_hexist("e1", "e1");
        let res = dispatch(cmd, &store);
        assert_res_ok(&res, &[false.into()], &[]);
    }

    #[test]
    fn hmexist_should_work() {
        let store = MemTable::new();

        set_pair_to_store("hm", vec![("k1", "v1"), ("k2", "v2"), ("k3", "v3")], &store);

        let cmd = CommandRequest::new_hmexist(
            "hm",
            vec!["k1".into(), "k2".into(), "k3".into(), "k4".into()],
        );
        let res = dispatch(cmd, &store);
        assert_res_ok(
            &res,
            &[true.into(), true.into(), true.into(), false.into()],
            &[],
        );
    }

    fn set_pair_to_store<T: Into<Value>>(table: &str, pairs: Vec<(&str, T)>, store: &impl Storage) {
        pairs
            .into_iter()
            .map(|(k, v)| CommandRequest::new_hset(table, k, v.into()))
            .for_each(|cmd| {
                dispatch(cmd, store);
            });
    }
}
