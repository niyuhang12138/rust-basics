use anyhow::{anyhow, Result};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use lazy_static::lazy_static;
use rand_core::OsRng;
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

    /// 用户注册: 在users表中存储argon2哈希过的密码
    pub async fn regsiter(&self, email: &str, password: &str) -> Result<i64> {
        let hashed_password = generate_password_hash(password)?;

        let id = sqlx::query("INSERT INTO users(email, hashed_password) VALUES(?, ?)")
            .bind(email)
            .bind(hashed_password)
            .execute(&self.poll)
            .await?
            .last_insert_rowid();

        Ok(id)
    }

    /// 用户登陆: 从users表中获取用户信息, 并用验证密码
    pub async fn login(&self, email: &str, password: &str) -> Result<String> {
        let user: User = sqlx::query_as("SELECT * from users WHERE email = ?")
            .bind(email)
            .fetch_one(&self.poll)
            .await?;

        println!("fin user: {user:?}");

        if let Err(_) = verify_password(password, &user.hashed_password) {
            return Err(anyhow!("failed to login"));
        }

        // 生成JWT token(此处省略JWT token的生成的细节)
        Ok("awesome token".into())
    }
}

/// 冲洗你创建users表
async fn recreate_table(poll: &SqlitePool) -> Result<()> {
    sqlx::query("DROP TABLE users").execute(poll).await?;
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS users(
            id      INTEGER     PRIMARY     KEY     NOT NULL,
            email   VARCHAR     UNIQUE      NOT NULL,
            HASHED_PASSWORD     VARCHAR     NOT NULL
        )"#,
    )
    .execute(poll)
    .await?;

    Ok(())
}

/// 创建安全的密码哈希
fn generate_password_hash(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| anyhow!("failed to hash password"))?
        .to_string())
}

/// 使用argon2验证用户密码和哈希
fn verify_password(password: &str, password_hash: &str) -> Result<()> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|_| anyhow!("failed to parse hashed password"))?;
    ARGON2
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| anyhow!("failed to verify password"))?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = env::var("DATBASE_URL").unwrap_or("sqlite:://./data/example.db".into());

    // 创建连接池
    let poll = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    // 每次运行都重新创建users表
    recreate_table(&poll).await?;

    let user_db = UserDb::new(poll.clone());
    let email = "nyh@163.com";
    let password = "nyh196511";

    // 新用户注册
    let id = user_db.regsiter(email, password).await?;
    println!("registered id: {id}");

    // 用户成功登陆
    let token = user_db.login(email, password).await?;
    println!("Login succeed: {token}");

    // 登陆失败
    let result = user_db.login(email, "dshdsiuchdsui").await;
    println!("Login should fail with bad password: {result:?}");

    Ok(())
}
