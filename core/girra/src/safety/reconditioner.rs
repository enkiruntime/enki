use crate::ast::expr::{BinaryOp, Expr, MatchArm};
use crate::ast::item::{FnDef, NamDef, Program, StructDef};
use crate::ast::stmt::Stmt;

#[derive(Debug, Default, Clone)]
pub struct Reconditioner {
    pub reconditioned_ops_count: usize,
}

impl Reconditioner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn recondition_program(&mut self, program: Program) -> Program {
        let structs = program.structs;
        let type_aliases = program.type_aliases;

        let functions = program
            .functions
            .into_iter()
            .map(|f| self.recondition_function(f))
            .collect();

        let nam_kernel = program.nam_kernel.map(|k| self.recondition_nam(k));

        Program {
            seed: program.seed,
            structs,
            type_aliases,
            functions,
            nam_kernel,
        }
    }

    pub fn recondition_function(&mut self, func: FnDef) -> FnDef {
        let body = func
            .body
            .into_iter()
            .map(|s| self.recondition_stmt(s))
            .collect();
        FnDef {
            name: func.name,
            args: func.args,
            return_type: func.return_type,
            body,
        }
    }

    pub fn recondition_nam(&mut self, nam: NamDef) -> NamDef {
        let body = nam
            .body
            .into_iter()
            .map(|s| self.recondition_stmt(s))
            .collect();
        NamDef {
            name: nam.name,
            space_param: nam.space_param,
            args: nam.args,
            body,
        }
    }

    pub fn recondition_stmt(&mut self, stmt: Stmt) -> Stmt {
        match stmt {
            Stmt::Let {
                is_mutable,
                name,
                ty,
                init,
            } => Stmt::Let {
                is_mutable,
                name,
                ty,
                init: self.recondition_expr(init),
            },
            Stmt::Assign { lhs, rhs } => Stmt::Assign {
                lhs: self.recondition_expr(lhs),
                rhs: self.recondition_expr(rhs),
            },
            Stmt::Expr {
                expr,
                has_semicolon,
            } => Stmt::Expr {
                expr: self.recondition_expr(expr),
                has_semicolon,
            },
            Stmt::Return(opt_e) => Stmt::Return(opt_e.map(|e| self.recondition_expr(e))),
            Stmt::Break => Stmt::Break,
            Stmt::Const { name, ty, value } => Stmt::Const {
                name,
                ty,
                value: self.recondition_expr(value),
            },
        }
    }

    pub fn recondition_expr(&mut self, expr: Expr) -> Expr {
        match expr {
            Expr::Binary { op, lhs, rhs } => {
                let safe_lhs = Box::new(self.recondition_expr(*lhs));
                let safe_rhs = Box::new(self.recondition_expr(*rhs));

                match op {
                    BinaryOp::Add => {
                        self.reconditioned_ops_count += 1;
                        Expr::WrappingAdd(safe_lhs, safe_rhs)
                    }
                    BinaryOp::Sub => {
                        self.reconditioned_ops_count += 1;
                        Expr::WrappingSub(safe_lhs, safe_rhs)
                    }
                    BinaryOp::Mul => {
                        self.reconditioned_ops_count += 1;
                        Expr::WrappingMul(safe_lhs, safe_rhs)
                    }
                    BinaryOp::Div => {
                        self.reconditioned_ops_count += 1;
                        let zero = Box::new(Expr::Int(0, "i32"));
                        Expr::SafeDiv(safe_lhs, safe_rhs, zero)
                    }
                    BinaryOp::Mod => {
                        self.reconditioned_ops_count += 1;
                        let zero = Box::new(Expr::Int(0, "i32"));
                        Expr::SafeMod(safe_lhs, safe_rhs, zero)
                    }
                    _ => Expr::Binary {
                        op,
                        lhs: safe_lhs,
                        rhs: safe_rhs,
                    },
                }
            }

            Expr::Index(arr, idx) => {
                self.reconditioned_ops_count += 1;
                let safe_arr = Box::new(self.recondition_expr(*arr));
                let safe_idx = Box::new(self.recondition_expr(*idx));
                Expr::SafeIndex(safe_arr, safe_idx)
            }

            Expr::Block(stmts, final_e) => {
                let safe_stmts = stmts
                    .into_iter()
                    .map(|s| self.recondition_stmt(s))
                    .collect();
                let safe_final = final_e.map(|e| Box::new(self.recondition_expr(*e)));
                Expr::Block(safe_stmts, safe_final)
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => Expr::If {
                cond: Box::new(self.recondition_expr(*cond)),
                then_branch: Box::new(self.recondition_expr(*then_branch)),
                else_branch: else_branch.map(|e| Box::new(self.recondition_expr(*e))),
            },
            Expr::Match { target, arms } => {
                let safe_target = Box::new(self.recondition_expr(*target));
                let safe_arms = arms
                    .into_iter()
                    .map(|arm| MatchArm {
                        pattern: arm.pattern,
                        guard: arm.guard.map(|g| Box::new(self.recondition_expr(*g))),
                        body: Box::new(self.recondition_expr(*arm.body)),
                    })
                    .collect();
                Expr::Match {
                    target: safe_target,
                    arms: safe_arms,
                }
            }
            Expr::Loop(stmts) => {
                let safe_stmts = stmts
                    .into_iter()
                    .map(|s| self.recondition_stmt(s))
                    .collect();
                Expr::Loop(safe_stmts)
            }

            Expr::TupleField(e, idx) => Expr::TupleField(Box::new(self.recondition_expr(*e)), idx),
            Expr::StructField(e, f) => Expr::StructField(Box::new(self.recondition_expr(*e)), f),
            Expr::Deref(e) => Expr::Deref(Box::new(self.recondition_expr(*e))),
            Expr::Ref(e) => Expr::Ref(Box::new(self.recondition_expr(*e))),
            Expr::RefMut(e) => Expr::RefMut(Box::new(self.recondition_expr(*e))),
            Expr::UnaryNot(e) => Expr::UnaryNot(Box::new(self.recondition_expr(*e))),
            Expr::UnaryNeg(e) => Expr::UnaryNeg(Box::new(self.recondition_expr(*e))),
            Expr::Cast(e, ty) => Expr::Cast(Box::new(self.recondition_expr(*e)), ty),
            Expr::Call { func_name, args } => Expr::Call {
                func_name,
                args: args.into_iter().map(|a| self.recondition_expr(a)).collect(),
            },
            Expr::MethodCall {
                receiver,
                method_name,
                args,
            } => Expr::MethodCall {
                receiver: Box::new(self.recondition_expr(*receiver)),
                method_name,
                args: args.into_iter().map(|a| self.recondition_expr(a)).collect(),
            },
            Expr::Tuple(elems) => Expr::Tuple(
                elems
                    .into_iter()
                    .map(|e| self.recondition_expr(e))
                    .collect(),
            ),
            Expr::Array(elems) => Expr::Array(
                elems
                    .into_iter()
                    .map(|e| self.recondition_expr(e))
                    .collect(),
            ),
            Expr::ArrayRepeat { elem, size } => Expr::ArrayRepeat {
                elem: Box::new(self.recondition_expr(*elem)),
                size,
            },
            Expr::StructInit {
                struct_name,
                fields,
            } => Expr::StructInit {
                struct_name,
                fields: fields
                    .into_iter()
                    .map(|(f_name, f_val)| (f_name, self.recondition_expr(f_val)))
                    .collect(),
            },

            other => other,
        }
    }
}
