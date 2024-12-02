# SQL查询工具

## SQL

我们工作的时候经常会跟各种数据源打交道, 数据源包括数据库, Parquet, CSV, JSON等, 而打交道的过程无非是: 数据获取(fetch), 过滤(filter), 投影(projection)和排序(sort)

做大数据的可以用类似Spart SQL的工具来完成各种异质数据的查询, 但是我们平时用SQL并没有这么强大, 因为虽然用SQL对数据库做查询, 任何DBMS都支持, 如果想用SQL查询CSV或者JSON就需要很多额外的处理

所以如果能有一个简单的工具, 不需要引入Spark, 就能支持对任何数据源使用SQL查询, 是不是很有意义?

比如你的shell支持这样使用是不是很舒服:

![image-20241202112521773](assets/image-20241202112521773.png)

在比如, 我们的客户端会从服务器API获取数据的子集, 如果这个子集可以在前端通过SQL直接做一些额外查询, 那将非常灵活, 并且用户可以得到即时的响应

软件领域有个著名的格林斯潘第十定律:

> 任何C或者Fortran程复杂到一定程度之后, 都会包含一个临时开发的, 不合规范的, 充满程序错误的, 运行速度很慢的, 只有一般功能的Common Lisp实现

我们仿照它来一个程序员的第四十二定律:

> 任何API接口复杂到一定程序后, 都会包含一个临时开发的, 不合规范的, 充满程序错误的, 运行速度很慢的, 只有一般功能的SQL实现

所以我们今天就来设计一个可以对任何数据源使用SQL查询, 并获得结果的库, 作为一个MVP, 我们就暂且只针对CSV的SQL查询, 不但如此, 我们还希望这个库可以给Python和Node使用

## 设计分析

我们首先需要一个SQL解析器, 在Rust下, 写一个解析器并不困单, 可以使用serde, 用任何parser combinator或者REG parser来实现, 比如nom或者pest, 不过SQL解析, 这种足够常见的需求, Rust社区已经有方案, 我们用sqlparser-rs

接下来就是如何把CSV或者其他数据源加载为DataFrame

做过数据处理或者使用过pandas的同学, 应该对DataFrame并不陌生, 它是一个矩阵数据结构, 其中每一列可能包含不同的类型, 可以在DataFrame上做过滤, 投影和排序等操作

在Rust下, 我们可以用polars, 来完成数据从CSV到DataFrame的加载和各种后续操作

确定了两个库之后, 后续的工作就是: 如何把sqlparser解析出来的抽象语法树AST, 映射到polars的DateFrame的操作上

抽象语法树是用来描述复杂语法规则的工具, 小大SQL或者某个DSL, 大到一门编程语言, 其语言结构都可以通过AST来描述, 如下图:

![image-20241202123749925](assets/image-20241202123749925.png)

如何把SQL语法和DataFrame的操作间进行映射呢? 比如我们要从数据中选出三列显示, 那这个`select a,b,c`就要能映射到DataFrame选取`a,b,c`三列输出

polars内部有自己的AST可以把各种操作聚合起来, 最后一并执行, 比如对于`where a > 10 and b < 5`, Polars的表达式是`col("a").gt(lit(10)).and(col("b").lt(let(5)))`, col代表列, gt/lt是大于/小于, lit是字面量的意思

有了这个认知, 对CSV等源进行SQL查询, 核心要解决的问题变成了, 如何把一个AST(SQL AST)转换成另一个AST(DataFrame AST)

这不就是宏编程做的事情么? 因为进一步分析二者的数据结构, 我们可以得到这样的对应关系:

![image-20241202124410931](assets/image-20241202124410931.png)

你看, 我们要做的主要事情就是, 在两个数据结构之间进行转换, 所以写完今天的代码, 你肯定会对宏有足够的信心

宏编程并没有什么大不了的, 抛开quote/unquote, 它主要的工作就是把一颗语法树转换成另外一颗语法树, 而这个转换的过程深入下去, 不过就是数据结构到数据结构的转换而已, 所以一句话总结: 宏编程主要流程就是实现若干From和TryFrom, 是不是很简单

当然, 这个转换的过程非常琐碎, 如果语言本身没有很好的模式匹配能力, 进行宏编程绝对是对自己非人道的这么

好在Rust有很棒的模式匹配支持, 它虽然你没有Erlang/Elixir的模式匹配那么强大, 但足以秒杀绝大多数的语言, 待会你在写的时候, 能直观感受到

## 创建一个SQL方言

好, 分析完要做额的事情, 接下来就是按部就班的写代码了

我们先创建一个库项目, 创建和src评级的examples, 并在`Cargo.toml`中添加代码:

```rust
[package]
name = "_05_queryer"
version = "0.1.0"
edition = "2021"

[[example]]
name = "dialect"

[dependencies]
anyhow = "1.0.93"
async-trait = "0.1.83"
polars = { version = "0.44.2", features = ["json", "lazy"] }
reqwest = { version = "0.12.9", default-features = false, features = ["rustls-tls", "json"] }
sqlparser = "0.52.0"
tokio = { version = "1.41.1", features = ["fs"] }

[dev-dependencies]
tokio = { version = "1.41.1", features = ["full"] }
tracing-subscriber = "0.3.19"
```

搞定依赖, 因为堆sqlparser的功能不太属性, 这里写个example尝试一下, 它会在examples目录下寻找`dialect.rs`文件

所以, 我们创建`examples/dialect.rs`文件, 并写一些测试sqlparser的代码

```rust
use sqlparser::{dialect::GenericDialect, parser::Parser};

fn main() {
    tracing_subscriber::fmt::init();
    let sql = "SELECT a a1, b, 123, myfunc(b), * \
  From data_source \
  WHERE a > b AND b < 100 AND c BETWEEN 10 AND 20 \
  ORDER BY a DESC, b \
  LIMIT 50 OFFSET 10";

    let ast = Parser::parse_sql(&GenericDialect::default(), sql);

    println!("{:#?}", ast);
}
```

这段代码用一个SQL语句来测试`Parser::parser_sql`会输出什么样的结构, 当你写库代码的时候, 如果遇到不明吧的第三方库, 可以用撰写example这种方式来先试一下

我们运行`cargo run --example dialect`查看结果:

```rust
Ok([Query(
    Query {
        with: None,
        body: Select(
            Select {
                distinct: false,
                top: None,
                projection: [ ... ],
                from: [ TableWithJoins { ... } ],
                selection: Some(BinaryOp { ... }),
                ...
            }
        ),
        order_by: [ OrderByExpr { ... } ],
        limit: Some(Value( ... )),
        offset: Some(Offset { ... })
    }
    ])
```

这里我们简化了一下, 你在命令行中看到, 会远比这个复杂

写到第九行的时候, 你有没有突发奇想, 如果SQL中的FROM字句后面可以接一个URL或者文件名多行? 这样我们就可以在URL或者文件中读取数据, 就像开头那个`select * from ps`的例子, 把ps命令作为数据源, 从它的输出中很方便的取数据

但是普通的SQL语句是不支持这种写法的, 不过sqlparser允许你创建自己的方案, 那我们就来尝试一下

创建`src/dialect.rs`文件, 写入下面的代码:

```rust
use sqlparser::dialect::Dialect;

#[derive(Debug, Default)]
pub struct TyrDialect;

// 创建自己的sql方言, TyrDialect支持identifier可以是简单的url
impl Dialect for TyrDialect {
    fn is_identifier_start(&self, ch: char) -> bool {
        ('a'..='z').contains(&ch) || ('A'..='Z').contains(&ch) || ch == '_'
    }

    // identifier可以有 ':', '/', '?', '&', '='
    fn is_identifier_part(&self, ch: char) -> bool {
        ('a'..='z').contains(&ch)
            || ('A'..='Z').contains(&ch)
            || ('0'..='9').contains(&ch)
            || [':', '/', '?', '&', '=', '-', '_', '.'].contains(&ch)
    }
}

/// 测试辅助函数
pub fn example_sql() -> String {
    let url = "https://raw.githubusercontent.com/owid/covid-19-data/master/public/data/latest/owid-covid-latest.csv";

    let sql: String = format!(
        "SELECT location name, total_cases, new_cases, total_deaths, new_deaths \
      FROM {url} where new_deaths >= 500 ORDER BY new_cases DESC LIMIT 6 OFFSET 5"
    );

    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::parser::Parser;

    #[test]
    fn it_works() {
        assert!(Parser::parse_sql(&TyrDialect::default(), &example_sql()).is_ok());
    }
}
```

这个代码主要实现了sqlparser的Dialect trait, 可以重载SQL解析器判断标识符的方法, 之后我们需要在`src/lib.rs`中添加:

```rust
mod dialect;
```

引入这个文件, 最后也写了一个测试, 你可以运行`cargo test`测试一下看看

测试通过! 现在我们就可以正常解析出这样的SQL了:

```rust
SELECT * from https://abc.xyz/covid-cases.csv where new_deaths >= 500SELECT * from https://abc.xyz/covid-cases.csv where new_deaths >= 500
```

因为通过trait, 你可以很方便的做控制反转, 在Rust中, 这是很常见的一件事情

## 实现AST的转换

刚刚完成了SQL解析, 解释就是用polars做AST转换了

由于我们不太了解polars库, 接下来还是先测试一下怎么用, 创建`examples/covid.rs`, 手动实现一个DataFrame的加载和查询:

```rust
use anyhow::Result;
use polars::prelude::*;
use std::io::Cursor;
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let url = "https://raw.githubusercontent.com/owid/covid-19-data/master/pub
    let data = reqwest::get(url).await?.text().await?;
    // 使用 polars 直接请求
    let df = CsvReader::new(Cursor::new(data))
    .infer_schema(Some(16))
    .finish()?;
    let filtered = df.filter(&df["new_deaths"].gt(500))?;
    println!(
        "{:?}",
        filtered.select((
            "location",
            "total_cases",
            "new_cases",
            "total_deaths",
            "new_deaths"
        ))
    );
    Ok(())
}
```

如果我们运行这个example, 可以得到一个打印的非常漂亮的表格, 

![image-20241202143834666](assets/image-20241202143834666.png)

我们最终要实现的就是这个效果, 通过解析一条类似查询的SQL, 来进行相同的数据查询, 该怎么做呢?

今天一开始我们就分析过了, 主要的工作就是把sqlparser解析出来的AST转换成polars定义的AST, 在回顾一下SQL AST的输出

```rust
Ok([Query(
    Query {
        with: None,
        body: Select(
            Select {
                distinct: false,
                top: None,
                projection: [ ... ],
                from: [ TableWithJoins { ... } ],
                selection: Some(BinaryOp { ... }),
                ...
            }
        ),
        order_by: [ OrderByExpr { ... } ],
        limit: Some(Value( ... )),
        offset: Some(Offset { ... })
    }
    ])
```

这里的Query是Statement enum其中一个结构, SQL语句除了查询外, 还有插入数据, 删除数据, 创建表等其他数据, 我们今天不关心这些, 只关心Query

所以, 可以创建一个文件`src/convert.rs`, 先定义一个数据结构Sql来描述两者的对应关系, 然后在实现Sql的TryFrom trait"