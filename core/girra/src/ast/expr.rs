use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    // Comparisons
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    // Logical
    And,
    Or,
}

impl BinaryOp {
    pub fn to_token_stream(self) -> TokenStream {
        match self {
            Self::Add => quote! { + },
            Self::Sub => quote! { - },
            Self::Mul => quote! { * },
            Self::Div => quote! { / },
            Self::Mod => quote! { % },
            Self::BitAnd => quote! { & },
            Self::BitOr => quote! { | },
            Self::BitXor => quote! { ^ },
            Self::Shl => quote! { << },
            Self::Shr => quote! { >> },
            Self::Eq => quote! { == },
            Self::Ne => quote! { != },
            Self::Gt => quote! { > },
            Self::Gte => quote! { >= },
            Self::Lt => quote! { < },
            Self::Lte => quote! { <= },
            Self::And => quote! { && },
            Self::Or => quote! { || },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: String,
    pub guard: Option<Box<Expr>>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Int(i64, &'static str),
    UInt(u64, &'static str),
    Float(String, &'static str),
    Bool(bool),
    Unit,

    Variable(String),
    TupleField(Box<Expr>, usize),
    StructField(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Deref(Box<Expr>),
    Ref(Box<Expr>),
    RefMut(Box<Expr>),

    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryNot(Box<Expr>),
    UnaryNeg(Box<Expr>),
    Cast(Box<Expr>, crate::types::Type),

    Call {
        func_name: String,
        args: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method_name: String,
        args: Vec<Expr>,
    },

    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    ArrayRepeat {
        elem: Box<Expr>,
        size: usize,
    },
    StructInit {
        struct_name: String,
        fields: Vec<(String, Expr)>,
    },

    Block(Vec<crate::ast::stmt::Stmt>, Option<Box<Expr>>),
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Loop(Vec<crate::ast::stmt::Stmt>),

    WrappingAdd(Box<Expr>, Box<Expr>),
    WrappingSub(Box<Expr>, Box<Expr>),
    WrappingMul(Box<Expr>, Box<Expr>),
    SafeDiv(Box<Expr>, Box<Expr>, Box<Expr>),
    SafeMod(Box<Expr>, Box<Expr>, Box<Expr>),
    SafeIndex(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::Int(val, suffix) => {
                let lit = syn::parse_str::<syn::LitInt>(&format!("{}{}", val, suffix))
                    .unwrap_or_else(|_| syn::parse_str("0i32").unwrap());
                quote! { #lit }
            }
            Self::UInt(val, suffix) => {
                let lit = syn::parse_str::<syn::LitInt>(&format!("{}{}", val, suffix))
                    .unwrap_or_else(|_| syn::parse_str("0u32").unwrap());
                quote! { #lit }
            }
            Self::Float(val_str, suffix) => {
                let lit = syn::parse_str::<syn::LitFloat>(&format!("{}{}", val_str, suffix))
                    .unwrap_or_else(|_| syn::parse_str("0.0f32").unwrap());
                quote! { #lit }
            }
            Self::Bool(b) => quote! { #b },
            Self::Unit => quote! { () },

            Self::Variable(name) => {
                if let Ok(ident) = syn::parse_str::<Ident>(name) {
                    quote! { #ident }
                } else {
                    let ts: TokenStream =
                        syn::parse_str(name).unwrap_or_else(|_| quote! { dummy_var });
                    quote! { #ts }
                }
            }
            Self::TupleField(expr, idx) => {
                let inner = expr.to_token_stream();
                let index = syn::Index::from(*idx);
                quote! { #inner.#index }
            }
            Self::StructField(expr, field) => {
                let inner = expr.to_token_stream();
                let field_ident = Ident::new(field, proc_macro2::Span::call_site());
                quote! { #inner.#field_ident }
            }
            Self::Index(arr, idx) => {
                let arr_t = arr.to_token_stream();
                let idx_t = idx.to_token_stream();
                quote! { #arr_t[#idx_t] }
            }
            Self::Deref(expr) => {
                let inner = expr.to_token_stream();
                quote! { (*#inner) }
            }
            Self::Ref(expr) => {
                let inner = expr.to_token_stream();
                quote! { &#inner }
            }
            Self::RefMut(expr) => {
                let inner = expr.to_token_stream();
                quote! { &mut #inner }
            }

            Self::Binary { op, lhs, rhs } => {
                let op_t = op.to_token_stream();
                let lhs_t = lhs.to_token_stream();
                let rhs_t = rhs.to_token_stream();
                quote! { (#lhs_t #op_t #rhs_t) }
            }
            Self::UnaryNot(expr) => {
                let inner = expr.to_token_stream();
                quote! { (!#inner) }
            }
            Self::UnaryNeg(expr) => {
                let inner = expr.to_token_stream();
                quote! { (-#inner) }
            }
            Self::Cast(expr, target_ty) => {
                let inner = expr.to_token_stream();
                let ty_t = target_ty.to_token_stream();
                quote! { (#inner as #ty_t) }
            }

            Self::Call { func_name, args } => {
                let func_t: TokenStream = syn::parse_str(func_name).unwrap_or_else(|_| {
                    let id = Ident::new(func_name, proc_macro2::Span::call_site());
                    quote! { #id }
                });
                let args_t: Vec<TokenStream> = args.iter().map(|a| a.to_token_stream()).collect();
                quote! { #func_t(#(#args_t),*) }
            }
            Self::MethodCall {
                receiver,
                method_name,
                args,
            } => {
                let rec_t = receiver.to_token_stream();
                let method_ident = Ident::new(method_name, proc_macro2::Span::call_site());
                let args_t: Vec<TokenStream> = args.iter().map(|a| a.to_token_stream()).collect();
                quote! { #rec_t.#method_ident(#(#args_t),*) }
            }

            Self::Tuple(elements) => {
                let elems_t: Vec<TokenStream> =
                    elements.iter().map(|e| e.to_token_stream()).collect();
                quote! { ( #(#elems_t),* ) }
            }
            Self::Array(elements) => {
                let elems_t: Vec<TokenStream> =
                    elements.iter().map(|e| e.to_token_stream()).collect();
                quote! { [ #(#elems_t),* ] }
            }
            Self::ArrayRepeat { elem, size } => {
                let elem_t = elem.to_token_stream();
                quote! { [#elem_t; #size] }
            }
            Self::StructInit {
                struct_name,
                fields,
            } => {
                let struct_ident = Ident::new(struct_name, proc_macro2::Span::call_site());
                let fields_t: Vec<TokenStream> = fields
                    .iter()
                    .map(|(f_name, f_val)| {
                        let f_ident = Ident::new(f_name, proc_macro2::Span::call_site());
                        let val_t = f_val.to_token_stream();
                        quote! { #f_ident: #val_t }
                    })
                    .collect();
                quote! { #struct_ident { #(#fields_t),* } }
            }

            Self::Block(stmts, final_expr) => {
                let stmts_t: Vec<TokenStream> = stmts.iter().map(|s| s.to_token_stream()).collect();
                let final_t = final_expr.as_ref().map(|e| e.to_token_stream());
                quote! {
                    {
                        #(#stmts_t)*
                        #final_t
                    }
                }
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_t = cond.to_token_stream();
                let then_t = then_branch.to_token_stream();

                let then_block = match **then_branch {
                    Expr::Block(..) => quote! { #then_t },
                    _ => quote! { { #then_t } },
                };

                if let Some(else_b) = else_branch {
                    let else_t = else_b.to_token_stream();
                    let else_block = match **else_b {
                        Expr::Block(..) => quote! { #else_t },
                        _ => quote! { { #else_t } },
                    };
                    quote! { (if #cond_t #then_block else #else_block) }
                } else {
                    quote! { (if #cond_t #then_block) }
                }
            }
            Expr::Match { target, arms } => {
                let target_t = target.to_token_stream();
                let arms_t: Vec<TokenStream> = arms
                    .iter()
                    .map(|arm| {
                        let pat_t: TokenStream =
                            syn::parse_str(&arm.pattern).unwrap_or_else(|_| quote! { _ });

                        let guard_t = arm.guard.as_ref().map(|g| {
                            let g_tokens = g.to_token_stream();
                            quote! { if #g_tokens }
                        });
                        let body_t = arm.body.to_token_stream();
                        quote! { #pat_t #guard_t => #body_t, }
                    })
                    .collect();
                quote! {
                    match #target_t {
                        #(#arms_t)*
                    }
                }
            }
            Self::Loop(stmts) => {
                let stmts_t: Vec<TokenStream> = stmts.iter().map(|s| s.to_token_stream()).collect();
                quote! {
                    loop {
                        #(#stmts_t)*
                    }
                }
            }

            Self::WrappingAdd(lhs, rhs) => {
                let l = lhs.to_token_stream();
                let r = rhs.to_token_stream();
                quote! { #l.wrapping_add(#r) }
            }
            Self::WrappingSub(lhs, rhs) => {
                let l = lhs.to_token_stream();
                let r = rhs.to_token_stream();
                quote! { #l.wrapping_sub(#r) }
            }
            Self::WrappingMul(lhs, rhs) => {
                let l = lhs.to_token_stream();
                let r = rhs.to_token_stream();
                quote! { #l.wrapping_mul(#r) }
            }
            Self::SafeDiv(lhs, rhs, fallback) => {
                let l = lhs.to_token_stream();
                let r = rhs.to_token_stream();
                let f = fallback.to_token_stream();
                quote! { ({ let d = #r; if d != #f { #l / d } else { #f } }) }
            }
            Self::SafeMod(lhs, rhs, fallback) => {
                let l = lhs.to_token_stream();
                let r = rhs.to_token_stream();
                let f = fallback.to_token_stream();
                quote! { ({ let d = #r; if d != #f { #l % d } else { #f } }) }
            }
            Self::SafeIndex(arr, idx) => {
                let a = arr.to_token_stream();
                let i = idx.to_token_stream();
                quote! { { let slice = &#a; slice[(#i as usize) % slice.len()] } }
            }
        }
    }
}
