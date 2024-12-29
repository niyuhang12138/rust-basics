use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{
    Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, GenericArgument, Path, Type,
    TypePath,
};

use darling::FromField;

#[derive(Debug, Default, FromField)]
#[darling(default, attributes(builder))]
struct Opts {
    each: Option<String>,
    default: Option<String>,
}

/// 我们需要的描述一个字段的所有信息
struct Fd {
    name: Ident,
    ty: Type,
    optional: bool,
    opts: Opts,
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
        // 从Field中去读attributes生成Opts, 如果没有使用缺省值
        let opts = Opts::from_field(&value).unwrap_or_default();
        Self {
            opts,
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

    // 为XXXBuilder生成相应的赋值语句, 把XXXBuilder每个字段赋值给XXX字段
    // 比如: #field_name: self.#field_name.take().ok_or("xxxx need to be set!")
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
}

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
