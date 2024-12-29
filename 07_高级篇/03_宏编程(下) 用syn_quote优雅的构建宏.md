# 宏编程(下): 用syn_quote优雅的构建宏

上一节我们用最原始的方式构建了RawBuilder派生宏, 本质就是从TokenStream中抽取需要的数据, 然后生成包含目标代码的字符串, 最后把字符串转换成TokenStream

说到解析TokenStream是个苦力活, 那么必然会有人做更好的工具, syn/quote这两个库就是Rust宏生态下处理TokenStream的解析以及代码生成很好用的库

今天我们就尝试用这个syn/quote工具, 来构建一个同样的Builder派生宏, 你可以对比一下两次的具体的实现感受syn/quote的方便之处

## syn crate简介

先看syn, syn是一个对TokenStream解析的库, 它提供了丰富的数据结构, 对语法树中遇到的各种Rust语言都有支持

比如一个Struct结构, 在TokenStream中, 看到的就是一个一系列TokenTree, 而通过syn解析后, struct的各种属性以及它的各种字段, 都有明确的类型, 这样我们可以很方便的通过模式匹配来选择合适的类型来进行对应的处理

syn还提供了对derive macro的特殊支持-DeriveInput类型

```rust
pub struct DeriveInput {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub ident: Ident,
    pub generics: Generics,
    pub data: Data,
}
```

通过DeriveInput类型, 我们可以很方便的解析派生宏, 比如这样

```rust
#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    ...
}
```

只需要使用`parse_macro_input!(input as DeriveInput)`, 我们就不必和TokenStream打交道, 而是使用解析出来的DeriveInput, 上一节我们从TokenStream里拿出来struct的名字, 都废了一番功夫, 这里直接访问DeriveInput的ident与就达到同样的目的, 非常人性化

## Parse trait

你也许会问: 为啥这个parse_macro_input有如此魔力? 我也可以使用它做类似的解析么?

要回答这个问题, 我们直接看代码找答案:

```rust
macro_rules! parse_macro_input {
    ($tokenstream:ident as $ty:ty) => {
        match $crate::parse_macro_input::parse::<$ty>($tokenstream) {
            $crate::__private::Ok(data) => data,
            $crate::__private::Err(err) => {
                return $crate::__private::TokenStream::from(err.to_compile_error());
            }
        }
    };
    ($tokenstream:ident with $parser:path) => {
        match $crate::parse::Parser::parse($parser, $tokenstream) {
            $crate::__private::Ok(data) => data,
            $crate::__private::Err(err) => {
                return $crate::__private::TokenStream::from(err.to_compile_error());
            }
        }
    };
    ($tokenstream:ident) => {
        $crate::parse_macro_input!($tokenstream as _)
    };
}
```

结合上一节的内容, 相信你不难理解, 如果我们调用`parse_macro_input!(input as DeriveInput)`, 实际上它执行了`$crate::parse::<DeriveInput>(input)`

那么这个pase函数究竟从何而来? 继续看代码:

```rust
pub fn parse<T: ParseMacroInput>(token_stream: TokenStream) -> Result<T> {
    T::parse.parse(token_stream)
}

pub trait ParseMacroInput: Sized {
    fn parse(input: ParseStream) -> Result<Self>;
}

impl<T: Parse> ParseMacroInput for T {
    fn parse(input: ParseStream) -> Result<Self> {
        <T as Parse>::parse(input)
    }
}
```

从这段代码我们得知, 任何实现了ParseMacroInput trait的类型T, 都支持parse函数, 进一步的, 任何T, 只要实现了Parse trait, 就自动实现了ParseMacroInput trait

而这个Parse trait, 就是一切魔法背后的源泉:

```rust
pub trait Parse: Sized {
    fn parse(input: ParseStream<'_>) -> Result<Self>;
}
```

syn下面几乎所有数据结构都实现了Parse trait, 包含DeriveInput, 所以如果我们想自己构建一个数据结构, 可以通过parse_macro_input!宏, 从TokenStream中读取内容, 并写入这个数据结构, 最好的方式就是就是为我们的数据结构实现Parse trait

关于Parse trait的使用, 今天就不深入下去了, 如果你感兴趣, 可以看看DeriveInput对Parse的实现, 你也可以进一步看看我们之前使用过的sqlx下的query!宏内部对Parse trait的实现

## quote crate简介

在宏编程的世界里, quote是一个特殊的原语, 它把代码转换成可以操作的数据(代码即数据), 看到这里, 你是不是想到了Lisp, 使得, quote这个概念来源于Lisp, 在Lisp里`(+ 1 2)`是代码, 而`'(+ 1 2)`是这个代码quote出来的数据

我们上一节在生成TokenStream的时候, 使用的是最原始的把包含代码的字符串转化成TokenStream的方法, 这种方法虽然可以通过使用模版很好的工作, 但在构建代码的过程中, 我们从操作的数据结构已经失去了语义

有没有办法让我们就像撰写整成的Rust代码一样, 保留所有的语义, 然后把他们转换成TokenStream?

有的, 可以使用quote crate, 它提供了一个quote!宏, 会替换代码中所有的`#(...)`, 生成TokenStream, 比如要写一个hello方法, 可以这样:

```rust
quote! {
    fn hello() {
        println!("Hello World!");
    }
}
```

这比字符串模版生成代码的方式更直观, 功能更强大, 而且保留代码的所有语义

quote!做替换的方式和macro_rules!非常类似, 也支持重复匹配, 一会在具体写代码的时候可以看到

## 用syn/quote重写Builder派生宏

现在我们对syn/quote有了一个粗浅的认识, 接下来就按照管理, 撰写代码更好的熟悉它们的功能

怎么做, 经过昨天的学习, 相信你现在也比较熟悉的了, 大致就是先从TokenStream抽取需要的数据, 在通过模版, 把抽取出来的数据转换成目标代码(TokenStream)

由于syn/quote生成的TokenStream是proc-macro2的类型, 所以我们还需要使用这个库, 简单说明一下proc-marcro2, 它是对proc-macro的简单封装, 使用起来更方便, 而且可以让过程宏可以单元测试

一下是我么需要添加的依赖

```toml
[package]
name = "macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
anyhow = "1.0.95"
askama = "0.12.1"
proc-macro2 = "1.0.92"
quote = "1.0.38"
syn = { version = "2.0.93", features = ["extra-traits"] }
```

注意anc crate默认所有的数据结构不带一些基本的trait, 比如Debug, 所以如果你想打印日志结构的话, 需要使用extra-traits feature

## Step1: 看看DeriveInputInput都输出什么?

在lib.rs中, 先添加新的Builder派生宏:

```rust
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    println!("{input:#?}");
    TokenStream::default()
}
```

通过parse_macro_input!, 我们得到了一个DeriveInput结构的数据, 这里可以打印一下, 看看会输出什么

所以在`examples/command.rs`中, 纤维Command引入Builder宏:

```rust
use macros::Builder;

#[derive(Builder)]
pub struct Command {
    executable: String,
    args: Vec<String>,
    env: Vec<String>,
    current_dir: Option<String>,
}

fn main() {
  
}
```

然后运行, 就可以看到详尽的DeriveInput的输出

```
DeriveInput {
    attrs: [],
    vis: Visibility::Public(
        Pub,
    ),
    ident: Ident {
        ident: "Command",
        span: #0 bytes(52..59),
    },
    generics: Generics {
        lt_token: None,
        params: [],
        gt_token: None,
        where_clause: None,
    },
    data: Data::Struct {
        struct_token: Struct,
        fields: Fields::Named {
            brace_token: Brace,
            named: [
                Field {
                    attrs: [],
                    vis: Visibility::Inherited,
                    mutability: FieldMutability::None,
                    ident: Some(
                        Ident {
                            ident: "executable",
                            span: #0 bytes(66..76),
                        },
                    ),
                    colon_token: Some(
                        Colon,
                    ),
                    ty: Type::Path {
                        qself: None,
                        path: Path {
                            leading_colon: None,
                            segments: [
                                PathSegment {
                                    ident: Ident {
                                        ident: "String",
                                        span: #0 bytes(78..84),
                                    },
                                    arguments: PathArguments::None,
                                },
                            ],
                        },
                    },
                },
                Comma,
                Field {
                    attrs: [],
                    vis: Visibility::Inherited,
                    mutability: FieldMutability::None,
                    ident: Some(
                        Ident {
                            ident: "args",
                            span: #0 bytes(90..94),
                        },
                    ),
                    colon_token: Some(
                        Colon,
                    ),
                    ty: Type::Path {
                        qself: None,
                        path: Path {
                            leading_colon: None,
                            segments: [
                                PathSegment {
                                    ident: Ident {
                                        ident: "Vec",
                                        span: #0 bytes(96..99),
                                    },
                                    arguments: PathArguments::AngleBracketed {
                                        colon2_token: None,
                                        lt_token: Lt,
                                        args: [
                                            GenericArgument::Type(
                                                Type::Path {
                                                    qself: None,
                                                    path: Path {
                                                        leading_colon: None,
                                                        segments: [
                                                            PathSegment {
                                                                ident: Ident {
                                                                    ident: "String",
                                                                    span: #0 bytes(100..106),
                                                                },
                                                                arguments: PathArguments::None,
                                                            },
                                                        ],
                                                    },
                                                },
                                            ),
                                        ],
                                        gt_token: Gt,
                                    },
                                },
                            ],
                        },
                    },
                },
                Comma,
                Field {
                    attrs: [],
                    vis: Visibility::Inherited,
                    mutability: FieldMutability::None,
                    ident: Some(
                        Ident {
                            ident: "env",
                            span: #0 bytes(113..116),
                        },
                    ),
                    colon_token: Some(
                        Colon,
                    ),
                    ty: Type::Path {
                        qself: None,
                        path: Path {
                            leading_colon: None,
                            segments: [
                                PathSegment {
                                    ident: Ident {
                                        ident: "Vec",
                                        span: #0 bytes(118..121),
                                    },
                                    arguments: PathArguments::AngleBracketed {
                                        colon2_token: None,
                                        lt_token: Lt,
                                        args: [
                                            GenericArgument::Type(
                                                Type::Path {
                                                    qself: None,
                                                    path: Path {
                                                        leading_colon: None,
                                                        segments: [
                                                            PathSegment {
                                                                ident: Ident {
                                                                    ident: "String",
                                                                    span: #0 bytes(122..128),
                                                                },
                                                                arguments: PathArguments::None,
                                                            },
                                                        ],
                                                    },
                                                },
                                            ),
                                        ],
                                        gt_token: Gt,
                                    },
                                },
                            ],
                        },
                    },
                },
                Comma,
                Field {
                    attrs: [],
                    vis: Visibility::Inherited,
                    mutability: FieldMutability::None,
                    ident: Some(
                        Ident {
                            ident: "current_dir",
                            span: #0 bytes(135..146),
                        },
                    ),
                    colon_token: Some(
                        Colon,
                    ),
                    ty: Type::Path {
                        qself: None,
                        path: Path {
                            leading_colon: None,
                            segments: [
                                PathSegment {
                                    ident: Ident {
                                        ident: "Option",
                                        span: #0 bytes(148..154),
                                    },
                                    arguments: PathArguments::AngleBracketed {
                                        colon2_token: None,
                                        lt_token: Lt,
                                        args: [
                                            GenericArgument::Type(
                                                Type::Path {
                                                    qself: None,
                                                    path: Path {
                                                        leading_colon: None,
                                                        segments: [
                                                            PathSegment {
                                                                ident: Ident {
                                                                    ident: "String",
                                                                    span: #0 bytes(155..161),
                                                                },
                                                                arguments: PathArguments::None,
                                                            },
                                                        ],
                                                    },
                                                },
                                            ),
                                        ],
                                        gt_token: Gt,
                                    },
                                },
                            ],
                        },
                    },
                },
                Comma,
            ],
        },
        semi_token: None,
    },
}
```

- 对于struct name, 可以直接从ident中获取
- 对于fields, 需要从data内部的DataStruct { fields }中获取, 目前我们只关心每个field的ident和ty

## Step2: 定义自己的用于处理derive宏的数据

和孩子前一样, 我们需要定义一个数据结构, 来获取TokenStream用到的信息

所以对比这上一讲, 可以定义如下的信息

```rust
use syn::{Ident, Type};

struct Fd {
  name: Ident,
  ty: Type,
  optional: bool,
}

pub struct BuilderContext {
  name: Ident,
  fields: Vec<Fd>,
}
```

## Step3: 把DeriveInputInput转换成自己的数据结构

接下来要做的是, 就是把DeriveInput转换成我们需要的BuilderContext

所以来写两个From trait, 分别把Field转换成Fd. DeriveInput转换成BuilderContext:

```rust
/// 把一个Field转换成Fd
impl From<Field> for Fd {
    fn from(value: Field) -> Self {
        let (optional, ty) = get_option_inner(value.ty);
        Self {
            name: value.ident.unwrap(),
            optional,
            ty,
        }
    }
}

/// 把DeriveInput转换成BuilderContext
impl From<DeriveInput> for BuilderContext {
    fn from(value: DeriveInput) -> Self {
        let name = value.ident;

        let fields = if let Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) = value.data
        {
            named
        } else {
            panic!("Unsupported data type");
        };

        let fds = fields.into_iter().map(Fd::from).collect();
        Self { name, fields: fds }
    }
}

/// 如果是T = Option<Inner>返回(true, Inner), 否则返回(false, T)
fn get_option_inner(ty: Type) -> (bool, Type) {
    todo!()
}
```

是不是简单的有点难以想象?

注意从input中获取fields时, 我们用了一个嵌套很深的模式匹配

```rust
if let Data::Struct(DataStruct {
    fields: Fields::Named(FieldsNamed { named, .. }),
    ..
}) = input.data
{
    named
}
```

如果没有强大的模式匹配支持, 获取FieldsNamed会是非常冗长的代码, 你可以仔细琢磨这两个From的实现, 它很好的体现了Rust的优雅

在处理Option类型的时候, 我们用了一个还不存在的get_option_inner函数, 这样一个函数是为了实现, 如果T = Option, 就返回(true, Inner), 否则就返回(false, T)

## Step4: 使用quote生成代码:

准备好BuilderContext, 就可以生成代码了, 来写一个render方法

```rust
impl BuilderContext {
    pub fn render(&self) -> TokenStream {
        let name = &self.name;
        // 生成XXXBuilder的ident
        let builder_name = Ident::new(&format!("{name}Builder"), name.span());

        let optionized_fields = self.gen_optionized_fields();
        let methods = self.gen_methods();
        let assigns = self.gen_assigns();

        quote! {
            /// Builder 结构
            #[derive(Debug, Default)]
            struct #builder_name {
                #(#optionized_fields,)*
            }

            /// Builder 结构每个字段赋值的方法，以及 build() 方法
            impl #builder_name {
                #(#methods)*

                pub fn build(mut self) -> Result<#name, &'static str> {
                    Ok(#name {
                        #(#assigns,)*
                    })
                }
            }

            /// 为使用 Builder 的原结构提供 builder() 方法，生成 Builder 结构
            impl #name {
                fn builder() -> #builder_name {
                    Default::default()
                }
            }
        }
    }

    // 为xxxBuilder生成Option<T>字段
    // 比如: executable: String -> executable: Option<String>
    fn gen_optionized_fields(&self) -> Vec<TokenStream> {
        todo!()
    }

    // 为XXXBuilder生成处理函数
    // 比如: methods: fn executable(mut self, v: impl Into<String>) -> Self { self.executable = Some(v); self }
    fn gen_methods(&self) -> Vec<TokenStream> {
        todo!()
    }

    // 为XXXBuilder生成相应的赋值语句, 把XXXBuilder每个字段赋值给XXX字段
    // 比如: #field_name: self.#field_name.take().ok_or("xxxx need to be set!")
    fn gen_assigns(&self) -> Vec<TokenStream> {
        todo!()
    }
}
```

可以看到, quote!包裹的代码, 和上一讲在template中都写的非常类似, 治国不循环的地方使用了quote!内部的重复语法`#(...)*`

到目前为止, 虽然我们的代码还不能运行, 但完整的从TokenStream到TokenStream转换的骨架已经完成, 剩下的只是实现细节而已, 你可以试着自己实现

## Step5: 完整实现

我们创建`src/builder.rs`文件. 填入一下代码:

```rust
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{
    Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, GenericArgument, Path, Type,
    TypePath,
};

/// 我们需要的描述一个字段的所有信息
struct Fd {
    name: Ident,
    ty: Type,
    optional: bool,
}

/// 我们需要的描述一个 struct 的所有信息
pub struct BuilderContext {
    name: Ident,
    fields: Vec<Fd>,
}

/// 把一个Field转换成Fd
impl From<Field> for Fd {
    fn from(value: Field) -> Self {
        let (optional, ty) = get_option_inner(&value.ty);
        Self {
            name: value.ident.unwrap(),
            optional,
            ty: ty.to_owned(),
        }
    }
}

/// 把DeriveInput转换成BuilderContext
impl From<DeriveInput> for BuilderContext {
    fn from(value: DeriveInput) -> Self {
        let name = value.ident;

        let fields = if let Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) = value.data
        {
            named
        } else {
            panic!("Unsupported data type");
        };

        let fds = fields.into_iter().map(Fd::from).collect();
        Self { name, fields: fds }
    }
}

impl BuilderContext {
    pub fn render(&self) -> TokenStream {
        let name = &self.name;
        // 生成XXXBuilder的ident
        let builder_name = Ident::new(&format!("{name}Builder"), name.span());

        let optionized_fields = self.gen_optionized_fields();
        let methods = self.gen_methods();
        let assigns = self.gen_assigns();

        quote! {
            /// Builder 结构
            #[derive(Debug, Default)]
            struct #builder_name {
                #(#optionized_fields,)*
            }

            /// Builder 结构每个字段赋值的方法，以及 build() 方法
            impl #builder_name {
                #(#methods)*

                pub fn build(mut self) -> Result<#name, &'static str> {
                    Ok(#name {
                        #(#assigns,)*
                    })
                }
            }

            /// 为使用 Builder 的原结构提供 builder() 方法，生成 Builder 结构
            impl #name {
                fn builder() -> #builder_name {
                    Default::default()
                }
            }
        }
    }

    // 为xxxBuilder生成Option<T>字段
    // 比如: executable: String -> executable: Option<String>
    fn gen_optionized_fields(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .map(|Fd { name, ty, .. }| quote! { #name: std::option::Option<#ty> })
            .collect()
    }

    // 为XXXBuilder生成处理函数
    // 比如: methods: fn executable(mut self, v: impl Into<String>) -> Self { self.executable = Some(v); self }
    fn gen_methods(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .map(|Fd { name, ty, .. }| {
                quote! {
                  pub fn #name(mut self, v: impl Into<#ty>) -> Self {
                    self.#name = Some(v.into());
                    self
                  }
                }
            })
            .collect()
    }

    // 为XXXBuilder生成相应的赋值语句, 把XXXBuilder每个字段赋值给XXX字段
    // 比如: #field_name: self.#field_name.take().ok_or("xxxx need to be set!")
    fn gen_assigns(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .map(|Fd { name, optional, .. }| {
                if *optional {
                    return quote! {
                      #name: self.#name.take()
                    };
                }

                quote! {
                  #name: self.#name.take().ok_or(concat!(stringify!(#name), " needs to be set!"))?
                }
            })
            .collect()
    }
}

/// 如果是T = Option<Inner>返回(true, Inner), 否则返回(false, T)
fn get_option_inner(ty: &Type) -> (bool, &Type) {
    // 首先模式匹配出 segments
    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(v) = segments.iter().next() {
            if v.ident == "Option" {
                // 如果 PathSegment 第一个是 Option，那么它内部应该是 AngleBracketed，比如 <T>
                // 获取其第一个值，如果是 GenericArgument::Type，则返回
                let t = match &v.arguments {
                    syn::PathArguments::AngleBracketed(a) => match a.args.iter().next() {
                        Some(GenericArgument::Type(t)) => t,
                        _ => panic!("Not sure what to do with other GenericArgument"),
                    },
                    _ => panic!("Not sure what to do with other PathArguments"),
                };
                return (true, t);
            }
        }
    }
    return (false, ty);
}
```

这段代码仔细阅读并不难理解, 可能get_option_inner拗口一些, 你需要对比DeriveInput的Debug信息对比这看着, 去推敲如何模式匹配

```
ty: Path(
    TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: [
                PathSegment {
                    ident: Ident {
                        ident: "Option",
                        span: #0 bytes(201..207),
                    },
                    arguments: AngleBracketed(
                        AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: Lt,
                            args: [
                                Type(
                                    Path(
                                        TypePath {
                                            qself: None,
                                            path: Path {
                                                leading_colon: None,
                                                segments: [
                                                    PathSegment {
                                                        ident: Ident {
                                                            ident: "String",
                                                            span: #0 bytes(208..214),
                                                        },
                                                        arguments: None,
                                                    },
                                                ],
                                            },
                                        },
                                    ),
                                ),
                            ],
                            gt_token: Gt,
                        },
                    ),
                },
            ],
        },
    },
),
```

这本身并不复杂, 难的是细心以及足够的耐心, 如果你对某个数据结构拿不准该怎么匹配, 可以在syn的文档中查找这个数据结构, 了解它的定义

好, 如果你理解了这个代码, 我们就可以更新`src/lib.rs`里定义的derive_builder了

```rust
mod builder;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    builder::BuilderContext::from(input).render().into()
}
```

可以从DeriveInput中生成一个BuilderContext, 然后render, 注意quote得到的是proc_macro2::TokenStream, 所以需要地哦啊用一下into转换成proc_macro::TokenStream

在`examples/command.rs`中, 更新Command的derive宏:

```rust
use macros::Builder;

#[derive(Builder, Debug)]
pub struct Command {
    executable: String,
    args: Vec<String>,
    env: Vec<String>,
    current_dir: Option<String>,
}

fn main() {}
```

## one more thing: 支持attributes

很多时候, 我们的派生宏可能还需要一些额外的attributes来提供更多信息, 更好的指导代码的生成, 比如serde, 你可以在数据结构中加入`#[serde(xxx)]attributes, 控制serde序列化/反序列化的行为

现在我们的Builder宏支持基本的功能, 但用着还不那么特别方便, 比如对于类型是Vec的args, 如果我们可以一次添加每个arg, 该多好

在proc-macro-workshop里Builder宏的第7个练习中, 就有这样一个要求:

```rust
#[derive(Builder)]
pub struct Command {
    executable: String,
    #[builder(each = "arg")]
    args: Vec<String>,
    #[builder(each = "env")]
    env: Vec<String>,
    current_dir: Option<String>,
}

fn main() {
    let command = Command::builder()
        .executable("cargo".to_owned())
        .arg("build".to_owned())
        .arg("--release".to_owned())
        .build()
        .unwrap();

    assert_eq!(command.executable, "cargo");
    assert_eq!(command.args, vec!["build", "--release"]);
}
```

这里, 如果字段定义了builder attributes, 并且剔红了each参数, 那么用户不断调用arg来依次添加参数, 这样使用起来, 就很方便了:

分析一下这个需求, 想要支持这样的功能, 首先要能够解析attributes, 然后要能够根据each attribute的内容生成对应的代码, 比如这样:

```rust
pub fn arg(mut self, v: String) -> Self {
    let mut data = self.args.take().unwrap_or_default();
    data.push(v);
    self.args = Some(data);
    self
}
```

syn提供的DeriveInput并没有对attributes额外处理, 所有的attributes被包裹在一个TokenTree::Group中

我们可以用上一节提到的方法, 手工处理TokenTree/TokenStream, 不过这样太麻烦, 社区里已经有一个非常棒的库加darling, 光是名字就听上去惹人怜爱, 用起来更是让人爱不释手, 我们就使用这个库, 来为Builder宏添加对attributes的支持

为了避免之前的Builder宏的破坏, 我们把`src/builder.rs`拷贝一份出来改名`src/builder_with_attr.rs`, 然后再`src/lib.rs`中引入它

在`src/lib.rs`中, 我们在创建创建一个BuilderWithAttrs的派生宏:

```rust
#[proc_macro_derive(BuilderWithAttr, attributes(builder))]
pub fn derive_builder_with_attr(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    builder_with_attr::BuilderContext::from(input)
        .render()
        .into()
}
```

和之前不同的是, 这里多了一个attributes(builder)属性, 这是告诉编译器, 请允许代码中出现`#[builder(,,,)]`, 它是我这个宏认识并要处理的

在创建一个`examples/command_with_attr.rs`, 把workshop中的代码粘进去并适当修改:

```rust
use macros::BuilderWithAttr;

#[allow(dead_code)]
#[derive(Debug, BuilderWithAttr)]
pub struct Command {
    executable: String,
    #[builder(each = "arg")]
    args: Vec<String>,
    #[builder(each = "env", default="vec![]")]
    env: Vec<String>,
    current_dir: Option<String>,
}

fn main() {
    let command = Command::builder()
        .executable("cargo".to_owned())
        .arg("build".to_owned())
        .arg("--release".to_owned())
        .build()
        .unwrap();

    assert_eq!(command.executable, "cargo");
    assert_eq!(command.args, vec!["build", "--release"]);
    println!("{:?}", command);
}
```

这里, 我们不仅希望支持each属性, 还支持default属性, 如果用户没有为这个域提供数据, 就使用default对应的代码来初始化

这个代码目前会报错, 因为并未有CommandBuilder添加arg方法, 接下来我们就要实现这个功能

在Cargo.toml中, 加入对darling的引用

```toml
[dependencies]
darling = "0.13"
```

然后在`src/builder_with_attr.rs`中, 添加用于捕获attributes的数据结构:

```rust
use darling::FromField;

#[derive(Debug, Default, FromField)]
#[darling(default, attributes(builder))]
struct Opts {
    each: Option<String>,
    default: Option<String>,
}
```

因为我们捕获的是field级别的attributes, 所以你这个结构需要海鲜FromField trait, 并且告诉draling要从那个attribute中获取(这里是从builder中获取)

不过现需要改动一下Fd, 让它包括Opts, 并且在From的实现中初始化opts

```rust
/// 我们需要的描述一个字段的所有信息
struct Fd {
    name: Ident,
    ty: Type,
    optional: bool,
    opts: Opts,
}

/// 把一个 Field 转换成 Fd
impl From<Field> for Fd {
    fn from(f: Field) -> Self {
        let (optional, ty) = get_option_inner(&f.ty);
        // 从 Field 中读取 attributes 生成 Opts，如果没有使用缺省值
        let opts = Opts::from_field(&f).unwrap_or_default();
        Self {
            opts,
            // 此时，我们拿到的是 NamedFields，所以 ident 必然存在
            name: f.ident.unwrap(),
            optional,
            ty: ty.to_owned(),
        }
    }
}
```

好现在Fd就包含了Opts的信息了, 我们可以利用这个信息来生成Methods和assigns

接下来看看gen_methods怎么修改, 如果Fd定义each attribute, 且它是个Vec的话, 我们就生成不一样的代码, 否则的话, 就先关来一样生成代码:

```rust
fn gen_methods(&self) -> Vec<TokenStream> {
    self.fields
    .iter()
    .map(|f| {
        let name = &f.name;
        let ty = &f.ty;
        // 如果不是 Option 类型，且定义了 each attribute
        if !f.optional && f.opts.each.is_some() {
            let each = Ident::new(f.opts.each.as_deref().unwrap(), name.span());
            let (is_vec, ty) = get_vec_inner(ty);
            if is_vec {
                return quote! {
                    pub fn #each(mut self, v: impl Into<#ty>) -> Self {
                        let mut data = self.#name.take().unwrap_or_default();
                        data.push(v.into());
                        self.#name = Some(data);
                        self
                    }
                };
            }
        }
        quote! {
            pub fn #name(mut self, v: impl Into<#ty>) -> Self {
                self.#name = Some(v.into());
                self
            }
        }
    })
    .collect()
}
```

这里, 我们重构了一下get_option_inner的代码, 因为get_vec_inner和它有相同的逻辑:

```rust
// 如果是 T = Option<Inner>，返回 (true, Inner)；否则返回 (false, T)
fn get_option_inner(ty: &Type) -> (bool, &Type) {
    get_type_inner(ty, "Option")
}

// 如果是 T = Vec<Inner>，返回 (true, Inner)；否则返回 (false, T)
fn get_vec_inner(ty: &Type) -> (bool, &Type) {
    get_type_inner(ty, "Vec")
}

fn get_type_inner<'a>(ty: &'a Type, name: &str) -> (bool, &'a Type) {
    // 首先模式匹配出 segments
    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(v) = segments.iter().next() {
            if v.ident == name {
                // 如果 PathSegment 第一个是 Option/Vec 等类型，那么它内部应该是 AngleBracketed，比如 <T>
                // 获取其第一个值，如果是 GenericArgument::Type，则返回
                let t = match &v.arguments {
                    syn::PathArguments::AngleBracketed(a) => match a.args.iter().next() {
                        Some(GenericArgument::Type(t)) => t,
                        _ => panic!("Not sure what to do with other GenericArgument"),
                    },
                    _ => panic!("Not sure what to do with other PathArguments"),
                };
                return (true, t);
            }
        }
    }
    return (false, ty);
}
```

最后, 我们为gen_assigns提供对default attribute的支持:

```rust
fn gen_assigns(&self) -> Vec<TokenStream> {
    self.fields
        .iter()
        .map(|Fd { name, optional, opts, .. }| {
            if *optional {
                return quote! {
                    #name: self.#name.take()
                };
            }

            // 如果定义了 default，那么把 default 里的字符串转换成 TokenStream
            // 使用 unwrap_or_else 在没有值的时候，使用缺省的结果
            if let Some(default) = opts.default.as_ref() {
                let ast: TokenStream = default.parse().unwrap();
                return quote! {
                    #name: self.#name.take().unwrap_or_else(|| #ast)
                };
            }

            quote! {
                #name: self.#name.take().ok_or(concat!(stringify!(#name), " needs to be set!"))?
            }
        })
        .collect()
}
```

## 小结

这一节, 我们使用syn/quote重写了builder派生宏的功能, 可以看到, 使用syn/quote后, 宏的开发变得简单很多, 最后我们还用darling进一步的提供了对attributes的支持

虽然这两讲我们只做了派生宏和一个非常简单的函数宏, 但是, 如果你学会了最复杂的派生宏, 那开发函数宏和属性宏也不再话下, 另外darling对attributes的支持, 同样也可以应用在属性宏中

