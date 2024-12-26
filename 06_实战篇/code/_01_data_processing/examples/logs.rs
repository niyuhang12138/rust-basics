use std::{
    io::{self, BufRead},
    sync::Arc,
};

use datafusion::{
    arrow::datatypes::{DataType, Field, Schema, SchemaRef},
    prelude::*,
};
use serde::{Deserialize, Serialize};

use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SchemaDataType {
    /// Int64
    Integer,
    /// Utf8
    String,
    /// Date64
    Date,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SchemaFiled {
    name: String,
    #[serde(rename = "type")]
    pub(crate) data_type: SchemaDataType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SchemaFields(Vec<SchemaFiled>);

impl From<SchemaDataType> for DataType {
    fn from(value: SchemaDataType) -> Self {
        match value {
            SchemaDataType::Integer => Self::Int64,
            SchemaDataType::String => Self::Utf8,
            SchemaDataType::Date => Self::Date64,
        }
    }
}

impl From<SchemaFiled> for Field {
    fn from(value: SchemaFiled) -> Self {
        Self::new(&value.name, value.data_type.into(), false)
    }
}

impl From<SchemaFields> for SchemaRef {
    fn from(value: SchemaFields) -> Self {
        let fields: Vec<Field> = value.0.into_iter().map(|f| f.into()).collect();

        Arc::new(Schema::new(fields))
    }
}

/// nginx日志处理的数据结构
pub struct NginxLog {
    ctx: SessionContext,
}

impl NginxLog {
    /// 根据schema定义, 数据文件以及分隔符构建NginxLog结构
    pub async fn try_new(schema_file: &str, data_file: &str, delim: u8) -> Result<Self> {
        let content = tokio::fs::read_to_string(schema_file).await?;
        let fields: SchemaFields = serde_yaml::from_str(&content)?;
        let schema = SchemaRef::from(fields);

        let mut ctx = SessionContext::new();
        let options = CsvReadOptions::new()
            .has_header(false)
            .delimiter(delim)
            .schema(&schema);
        ctx.register_csv("nginx", data_file, options).await?;

        Ok(Self { ctx })
    }

    /// 进行sql查询
    pub async fn query(&mut self, query: &str) -> Result<DataFrame> {
        let df: DataFrame = self.ctx.sql(query).await?;
        Ok(df)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("1");
    let mut nginx_log =
        NginxLog::try_new("fixtures/log_schema.yml", "fixtures/logs.csv", b' ').await?;
    println!("2");
    // 从stdin中按行读取, 当做sql查询进行处理
    let stdin = io::stdin();
    println!("3");

    let mut lines = stdin.lock().lines();
    println!("4");

    while let Some(Ok(line)) = lines.next() {
        println!("5");
        if !line.starts_with("--") {
            println!("6");
            println!("{line}");
            // 读到一行sql, 查询, 获取dataframe
            let df = nginx_log.query(&line).await?;
            df.show().await?;
        }
        println!("7");
    }
    println!("8");

    Ok(())
}
