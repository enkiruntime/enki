use crate::ast::expr::{BinaryOp, Expr};
use crate::ast::item::{FnDef, NamDef, Program, StructDef};
use crate::ast::stmt::Stmt;
use crate::context::{Context, NodeKind};
use crate::generator::ident::IdentGenerator;
use crate::generator::policy::FuzzPolicy;
use crate::generator::weights::{ExprCategory, SelectionWeights, StmtKind};
use crate::safety::Reconditioner;
use crate::symbol::{FunctionSignature, SymbolTable};
use crate::types::{
    FloatType, IntType, PrimitiveType, StructField, StructType, TupleType, Type, UIntType,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub struct ASTGenerator {
    pub rng: ChaCha8Rng,
    pub idents: IdentGenerator,
    pub symbols: SymbolTable,
    pub policy: FuzzPolicy,
    pub reconditioner: Reconditioner,
}

impl ASTGenerator {
    pub fn new(seed: u64, policy: FuzzPolicy) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            idents: IdentGenerator::new(),
            symbols: SymbolTable::new(),
            policy,
            reconditioner: Reconditioner::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Program & Kernel Synthesis
    // -------------------------------------------------------------------------

    pub fn generate_program(&mut self, seed: u64) -> Program {
        // 1. توليد Structs وتسجيلها في الـ Registry
        let mut structs = Vec::new();
        if self.policy.allow_structs {
            let struct_count = self.rng.gen_range(2..=4);
            for _ in 0..struct_count {
                let st = self.generate_struct_def();
                structs.push(st);
            }
        }

        // 2. توليد دوال مساعدة وتسجيلها في الـ Registry لتكون قابلة للاستدعاء
        let mut functions = Vec::new();
        let fn_count = self.rng.gen_range(2..=3);
        for _ in 0..fn_count {
            let f = self.generate_helper_function();
            functions.push(f);
        }

        // 3. توليد دالة GPU Kernel الرئيسية التي تستخدم الـ Structs وتستدعي الدوال
        let nam_kernel = self.generate_nam_kernel("fuzz_target");

        let raw_program = Program {
            seed,
            structs,
            type_aliases: Vec::new(),
            functions,
            nam_kernel: Some(nam_kernel),
        };

        self.reconditioner.recondition_program(raw_program)
    }

    pub fn generate_nam_kernel(&mut self, kernel_name: &str) -> NamDef {
        self.symbols.enter_scope();
        let ctx = Context::new(kernel_name);

        let pixel_ref_type = Type::Reference(crate::types::ReferenceType {
            element_type: Box::new(Type::u32()),
            mutable: true,
            lifetime_id: None,
        });
        self.symbols.insert("pixel", pixel_ref_type.clone(), true);

        let mut body_stmts = Vec::new();

        if let Some(st) = self.symbols.registry.get_random_struct(&mut self.rng) {
            let st_var_name = self.idents.next_var();
            let st_type = Type::Struct(st.clone());
            let st_init = self.generate_literal(&st_type);
            self.symbols.insert(&st_var_name, st_type.clone(), false);
            body_stmts.push(Stmt::Let {
                is_mutable: false,
                name: st_var_name.clone(),
                ty: Some(st_type),
                init: st_init,
            });
        }

        if let Some(fn_sig) = self.symbols.registry.functions.first().cloned() {
            let res_var_name = self.idents.next_var();
            let args: Vec<Expr> = fn_sig
                .args
                .iter()
                .map(|(_, arg_ty)| self.generate_expression(arg_ty, &ctx))
                .collect();
            let call_expr = Expr::Call {
                func_name: fn_sig.name.clone(),
                args,
            };
            self.symbols
                .insert(&res_var_name, fn_sig.return_type.clone(), false);
            body_stmts.push(Stmt::Let {
                is_mutable: false,
                name: res_var_name,
                ty: Some(fn_sig.return_type.clone()),
                init: call_expr,
            });
        }

        let extra_stmts = self.generate_statement_block(&ctx, 4);
        body_stmts.extend(extra_stmts);

        let final_val = self.generate_expression(&Type::u32(), &ctx);
        body_stmts.push(Stmt::Assign {
            lhs: Expr::Deref(Box::new(Expr::Variable("pixel".into()))),
            rhs: final_val,
        });

        self.symbols.exit_scope();

        let args = vec![("pixel".into(), pixel_ref_type)];

        NamDef {
            name: kernel_name.into(),
            space_param: "space".into(),
            args,
            body: body_stmts,
        }
    }

    // -------------------------------------------------------------------------
    // Statement Generation
    // -------------------------------------------------------------------------

    pub fn generate_statement_block(&mut self, ctx: &Context, target_count: usize) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        let count = self.rng.gen_range(3..=target_count.max(3));

        for _ in 0..count {
            let stmt = self.generate_statement(ctx);
            stmts.push(stmt);
        }

        stmts
    }

    pub fn generate_statement(&mut self, ctx: &Context) -> Stmt {
        let kind = SelectionWeights::select_stmt_kind(ctx, &self.policy, &mut self.rng);

        match kind {
            StmtKind::Let => self.generate_let_statement(ctx),
            StmtKind::Assign => self.generate_assign_statement(ctx),
            StmtKind::Expr => {
                let ty = self.generate_random_type();
                let expr = self.generate_expression(&ty, ctx);
                Stmt::Expr {
                    expr,
                    has_semicolon: true,
                }
            }
            StmtKind::Return => {
                let ret_ty = ctx.return_expression_type.clone().unwrap_or_else(Type::i32);
                let expr = self.generate_expression(&ret_ty, ctx);
                Stmt::Return(Some(expr))
            }
            StmtKind::Break => Stmt::Break,
        }
    }

    pub fn generate_let_statement(&mut self, ctx: &Context) -> Stmt {
        let var_name = self.idents.next_var();
        let is_mutable = self.rng.gen_bool(0.4);
        let ty = self.generate_random_type();

        let init = self.generate_expression(&ty, ctx);
        self.symbols.insert(&var_name, ty.clone(), is_mutable);

        Stmt::Let {
            is_mutable,
            name: var_name,
            ty: Some(ty),
            init,
        }
    }

    pub fn generate_assign_statement(&mut self, ctx: &Context) -> Stmt {
        let assignable_vars = self.symbols.get_assignable_variables();

        if let Some(target_var) = assignable_vars.first() {
            let var_name = target_var.name.clone();
            let var_ty = target_var.ty.clone();

            // إذا كان المتغير مرجعاً قابلاً للتعديل مثل pixel (&mut u32)، نسند للقيمة عبر Dereference
            match var_ty {
                Type::Reference(ref r) if r.mutable => {
                    let rhs = self.generate_expression(&r.element_type, ctx);
                    Stmt::Assign {
                        lhs: Expr::Deref(Box::new(Expr::Variable(var_name))),
                        rhs,
                    }
                }
                _ => {
                    let rhs = self.generate_expression(&var_ty, ctx);
                    Stmt::Assign {
                        lhs: Expr::Variable(var_name),
                        rhs,
                    }
                }
            }
        } else {
            self.generate_let_statement(ctx)
        }
    }

    // -------------------------------------------------------------------------
    // Expression Generation (ربط الدوال والاستدعاءات والـ Structs)
    // -------------------------------------------------------------------------

    pub fn generate_expression(&mut self, target_type: &Type, ctx: &Context) -> Expr {
        let category = SelectionWeights::select_expr_category(ctx, &self.policy, &mut self.rng);

        match category {
            ExprCategory::Literal => self.generate_literal(target_type),

            ExprCategory::Variable => {
                let available = self.symbols.find_variables_of_type(target_type, false);
                if let Some(var) = available.first() {
                    Expr::Variable(var.name.clone())
                } else {
                    self.generate_literal(target_type)
                }
            }

            // 1. استدعاء الدوال المساعدة المتاحة (Call Expression)
            ExprCategory::Call => {
                if let Some(fn_sig) = self
                    .symbols
                    .registry
                    .get_random_function_returning(target_type, &mut self.rng)
                {
                    let next_ctx = ctx.increment_depth(NodeKind::FunctionCall);
                    let args = fn_sig
                        .args
                        .iter()
                        .map(|(_, arg_ty)| self.generate_expression(arg_ty, &next_ctx))
                        .collect();

                    Expr::Call {
                        func_name: fn_sig.name,
                        args,
                    }
                } else {
                    self.generate_literal(target_type)
                }
            }

            // 2. إنشاء Struct كامل بحقوله (Struct Initialization)
            ExprCategory::StructInit if matches!(target_type, Type::Struct(_)) => {
                if let Type::Struct(s) = target_type {
                    let next_ctx = ctx.increment_depth(NodeKind::StructType);
                    let fields = s
                        .fields
                        .iter()
                        .map(|f| (f.name.clone(), self.generate_expression(&f.ty, &next_ctx)))
                        .collect();
                    Expr::StructInit {
                        struct_name: s.name.clone(),
                        fields,
                    }
                } else {
                    self.generate_literal(target_type)
                }
            }

            // 3. إنشاء Tuple كامل بعناصره
            ExprCategory::Tuple if matches!(target_type, Type::Tuple(_)) => {
                if let Type::Tuple(t) = target_type {
                    let next_ctx = ctx.increment_depth(NodeKind::TupleType);
                    let elements = t
                        .elements
                        .iter()
                        .map(|el_ty| self.generate_expression(el_ty, &next_ctx))
                        .collect();
                    Expr::Tuple(elements)
                } else {
                    self.generate_literal(target_type)
                }
            }

            // 4. العمليات الحسابية والمنطقية
            ExprCategory::Binary
                if target_type.is_numeric() || target_type.is_bitwise_compatible() =>
            {
                let next_ctx = ctx.increment_depth(NodeKind::RecursiveExpr);
                let lhs = Box::new(self.generate_expression(target_type, &next_ctx));
                let rhs = Box::new(self.generate_expression(target_type, &next_ctx));

                if target_type.is_integer() {
                    let ops = [
                        BinaryOp::Add,
                        BinaryOp::Sub,
                        BinaryOp::Mul,
                        BinaryOp::Div,
                        BinaryOp::Mod,
                        BinaryOp::BitAnd,
                        BinaryOp::BitOr,
                        BinaryOp::BitXor,
                    ];
                    let op = ops[self.rng.gen_range(0..ops.len())];
                    match op {
                        BinaryOp::Add => Expr::WrappingAdd(lhs, rhs),
                        BinaryOp::Sub => Expr::WrappingSub(lhs, rhs),
                        BinaryOp::Mul => Expr::WrappingMul(lhs, rhs),
                        BinaryOp::Div => {
                            let zero = Box::new(target_type.zero_expr());
                            Expr::SafeDiv(lhs, rhs, zero)
                        }
                        BinaryOp::Mod => {
                            let zero = Box::new(target_type.zero_expr());
                            Expr::SafeMod(lhs, rhs, zero)
                        }
                        _ => Expr::Binary { op, lhs, rhs },
                    }
                } else if target_type.is_float() {
                    // الـ Floats تستخدم + و - و * عادية بدون wrapping
                    let ops = [BinaryOp::Add, BinaryOp::Sub, BinaryOp::Mul, BinaryOp::Div];
                    let op = ops[self.rng.gen_range(0..ops.len())];
                    if op == BinaryOp::Div {
                        let zero = Box::new(target_type.zero_expr());
                        Expr::SafeDiv(lhs, rhs, zero)
                    } else {
                        Expr::Binary { op, lhs, rhs }
                    }
                } else {
                    let ops = [
                        BinaryOp::And,
                        BinaryOp::Or,
                        BinaryOp::BitAnd,
                        BinaryOp::BitOr,
                    ];
                    let op = ops[self.rng.gen_range(0..ops.len())];
                    Expr::Binary { op, lhs, rhs }
                }
            }

            ExprCategory::IfElse => {
                let next_ctx = ctx.increment_depth(NodeKind::IfElse);
                let cond = Box::new(self.generate_expression(&Type::bool(), &next_ctx));
                let then_b = Box::new(self.generate_expression(target_type, &next_ctx));
                let else_b = Box::new(self.generate_expression(target_type, &next_ctx));
                Expr::If {
                    cond,
                    then_branch: then_b,
                    else_branch: Some(else_b),
                }
            }

            ExprCategory::Block => {
                self.symbols.enter_scope();
                let next_ctx = ctx.enter_scope().increment_depth(NodeKind::Block);
                let stmts = self.generate_statement_block(&next_ctx, 3);
                let final_e = Box::new(self.generate_expression(target_type, &next_ctx));
                self.symbols.exit_scope();
                Expr::Block(stmts, Some(final_e))
            }

            // الـ Cast مسموح فقط بين الأرقام، وممنوع التحويل إلى bool عبر `as`
            ExprCategory::Cast
                if target_type.is_numeric() && !target_type.is_bitwise_compatible() =>
            {
                let src_type = self.generate_primitive_type();
                if src_type.is_numeric() && src_type != *target_type {
                    let next_ctx = ctx.increment_depth(NodeKind::RecursiveExpr);
                    let inner = Box::new(self.generate_expression(&src_type, &next_ctx));
                    Expr::Cast(inner, target_type.clone())
                } else {
                    self.generate_literal(target_type)
                }
            }

            _ => self.generate_literal(target_type),
        }
    }

    pub fn generate_literal(&mut self, target_type: &Type) -> Expr {
        match target_type {
            Type::Primitive(PrimitiveType::Int(IntType::I8)) => {
                Expr::Int(self.rng.gen_range(-128..=127), "i8")
            }
            Type::Primitive(PrimitiveType::Int(IntType::I16)) => {
                Expr::Int(self.rng.gen_range(-32768..=32767), "i16")
            }
            Type::Primitive(PrimitiveType::Int(IntType::I32)) => {
                Expr::Int(self.rng.gen_range(-100000..=100000), "i32")
            }
            Type::Primitive(PrimitiveType::Int(IntType::I64)) => {
                Expr::Int(self.rng.gen_range(-1000000..=1000000), "i64")
            }
            Type::Primitive(PrimitiveType::UInt(UIntType::U8)) => {
                Expr::UInt(self.rng.gen_range(0..=255), "u8")
            }
            Type::Primitive(PrimitiveType::UInt(UIntType::U16)) => {
                Expr::UInt(self.rng.gen_range(0..=65535), "u16")
            }
            Type::Primitive(PrimitiveType::UInt(UIntType::U32)) => {
                Expr::UInt(self.rng.gen_range(0..=200000), "u32")
            }
            Type::Primitive(PrimitiveType::UInt(UIntType::U64)) => {
                Expr::UInt(self.rng.gen_range(0..=2000000), "u64")
            }
            Type::Primitive(PrimitiveType::UInt(UIntType::USize)) => {
                Expr::UInt(self.rng.gen_range(0..=1024), "usize")
            }
            Type::Primitive(PrimitiveType::Float(FloatType::F32)) => {
                let val: f32 = self.rng.gen_range(-1000.0..1000.0);
                Expr::Float(format!("{:.4}", val), "f32")
            }
            Type::Primitive(PrimitiveType::Float(FloatType::F64)) => {
                let val: f64 = self.rng.gen_range(-1000.0..1000.0);
                Expr::Float(format!("{:.4}", val), "f64")
            }
            Type::Primitive(PrimitiveType::Bool) => Expr::Bool(self.rng.gen_bool(0.5)),
            Type::Primitive(PrimitiveType::Unit) => Expr::Unit,
            Type::Tuple(t) => {
                let elems = t
                    .elements
                    .iter()
                    .map(|elem_ty| self.generate_literal(elem_ty))
                    .collect();
                Expr::Tuple(elems)
            }
            Type::Struct(s) => {
                let fields = s
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), self.generate_literal(&f.ty)))
                    .collect();
                Expr::StructInit {
                    struct_name: s.name.clone(),
                    fields,
                }
            }
            _ => Expr::Int(0, "i32"),
        }
    }

    // -------------------------------------------------------------------------
    // Types & Structs Synthesis
    // -------------------------------------------------------------------------

    pub fn generate_random_type(&mut self) -> Type {
        let pick = self.rng.gen_range(0..10);
        match pick {
            0..=5 => self.generate_primitive_type(),
            6..=7 if self.policy.allow_tuples => {
                let count = self.rng.gen_range(2..=4);
                let elems = (0..count).map(|_| self.generate_primitive_type()).collect();
                Type::Tuple(TupleType { elements: elems })
            }
            8..=9 if self.policy.allow_structs => {
                if let Some(st) = self.symbols.registry.get_random_struct(&mut self.rng) {
                    Type::Struct(st)
                } else {
                    self.generate_primitive_type()
                }
            }
            _ => self.generate_primitive_type(),
        }
    }

    pub fn generate_primitive_type(&mut self) -> Type {
        let choices = [
            Type::i8(),
            Type::i16(),
            Type::i32(),
            Type::i64(),
            Type::u8(),
            Type::u16(),
            Type::u32(),
            Type::u64(),
            Type::f32(),
            Type::f64(),
            Type::bool(),
        ];
        choices[self.rng.gen_range(0..choices.len())].clone()
    }

    pub fn generate_struct_def(&mut self) -> StructDef {
        let struct_name = self.idents.next_struct();
        let field_count = self.rng.gen_range(2..=4);
        let mut fields = Vec::new();

        for i in 0..field_count {
            let f_name = format!("field_{}", i);
            let f_ty = self.generate_primitive_type();
            fields.push(StructField {
                name: f_name,
                ty: f_ty,
            });
        }

        let st_type = StructType {
            name: struct_name.clone(),
            fields: fields.clone(),
        };
        // تسجيل الـ Struct في السجل العام لكي تستخدمه الدوال الأخرى
        self.symbols.registry.register_struct(st_type);

        StructDef {
            name: struct_name,
            fields,
            is_repr_c: true,
        }
    }

    pub fn generate_helper_function(&mut self) -> FnDef {
        let fn_name = self.idents.next_fn();
        let ret_ty = self.generate_primitive_type();
        let arg_count = self.rng.gen_range(1..=3);

        self.symbols.enter_scope();
        let mut args = Vec::new();
        for i in 0..arg_count {
            let a_name = format!("arg_{}", i);
            let a_ty = self.generate_primitive_type();
            self.symbols.insert(&a_name, a_ty.clone(), false);
            args.push((a_name, a_ty));
        }

        // تسجيل الدالة في الـ Global Registry لتتمكن الدوال الأخرى من استدعائها!
        self.symbols.registry.register_function(FunctionSignature {
            name: fn_name.clone(),
            args: args.clone(),
            return_type: ret_ty.clone(),
        });

        let ctx = Context::new(&fn_name).with_return_type(ret_ty.clone());
        let mut body = self.generate_statement_block(&ctx, 3);

        // إضافة تعبير إرجاع نهائي مطابق للـ return_type
        let return_val = self.generate_expression(&ret_ty, &ctx);
        body.push(Stmt::Expr {
            expr: return_val,
            has_semicolon: false,
        });

        self.symbols.exit_scope();

        FnDef {
            name: fn_name,
            args,
            return_type: ret_ty,
            body,
        }
    }

    fn choose_binary_op(&mut self, ty: &Type) -> BinaryOp {
        if ty.is_integer() {
            let ops = [
                BinaryOp::Add,
                BinaryOp::Sub,
                BinaryOp::Mul,
                BinaryOp::Div,
                BinaryOp::Mod,
                BinaryOp::BitAnd,
                BinaryOp::BitOr,
                BinaryOp::BitXor,
            ];
            ops[self.rng.gen_range(0..ops.len())]
        } else if ty.is_float() {
            let ops = [BinaryOp::Add, BinaryOp::Sub, BinaryOp::Mul, BinaryOp::Div];
            ops[self.rng.gen_range(0..ops.len())]
        } else {
            let ops = [
                BinaryOp::And,
                BinaryOp::Or,
                BinaryOp::BitAnd,
                BinaryOp::BitOr,
            ];
            ops[self.rng.gen_range(0..ops.len())]
        }
    }
}
