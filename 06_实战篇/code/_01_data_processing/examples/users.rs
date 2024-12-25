use anyhow::Result;
use argon2::Argon2;
use lazy_static::lazy_static;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use std::env;

/// Argon2 hash使用的密码
const ARGON_SECRET: &[u8] = b"deadbeef";
lazy_static! {
  /// Argon2
  static ref ARGON2: Argon2<'static> = Argon2::new_with_secret(ARGON_SECRET, argon2::Algorithm::default(), argon2::Version::default(), argon2::Params::default()).unwrap();
}

/// user表对应的数据结构, 处理login/register
pub struct UserDb {
    poll: SqlitePool,
}

/// 使用FromRow派生宏从数据库中读取出来的数据结构转成User
#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]

pub struct User {
    id: i64,
    email: String,
    hashed_password: String,
}

impl UserDb {
    pub fn new(poll: SqlitePool) -> Self {
        Self { poll }
    }

    // /// 用户注册: 在users表中存储argon2哈希过的密码
    // pub async fn regsi
}

fn main() {}
