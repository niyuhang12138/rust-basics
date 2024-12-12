mod command_service;

pub use command_service::*;

use crate::*;

/// 对Command的处理抽象
pub trait CommandService {
    fn execute(self, store: &impl Storage) -> CommandResponse;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn service_should_works() {
        // 我们需要一个service结构只要包含Storage
        let service = Service::new(MemTable::default());

        // service可以运行在多线程的环境下, 它的clone应该是轻量的
        let cloned = service.clone();

        // 创建一个线程, 在table t1中写入k1, v1
        let handle = std::thread::spawn(move || {
            let res = cloned.execute(CommandRequest::new_hset("t1", "k1", "v1".into()));
            assert_res_ok(res, &[Value::default()], &[]);
        });

        handle.join().unwrap();

        // 在档期你咸亨下读取table t1的k1, 应该返回v1
        let res = service.execute(CommandRequest::new_hget("t1", "k1"));
        assert_res_ok(res, &["v1".into()], &[]);
    }
}

#[cfg(test)]
// 测试成功返回的结果
pub fn assert_res_ok(mut res: CommandResponse, values: &[Value], pairs: &[Kvpair]) {
    res.pairs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(res.status, 200);
    assert_eq!(res.message, "");
    assert_eq!(res.values, values);
    assert_eq!(res.pairs, pairs)
}

// 测试失败返回的结果
pub fn assert_res_error(res: CommandResponse, code: u32, msg: &str) {
    assert_eq!(res.status, code);
    assert!(res.message.contains(msg));
    assert_eq!(res.values, &[]);
    assert_eq!(res.pairs, &[]);
}
