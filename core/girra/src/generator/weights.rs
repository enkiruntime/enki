use crate::context::Context;
use crate::generator::policy::FuzzPolicy;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StmtKind {
    Let,
    Assign,
    Expr,
    Return,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprCategory {
    Literal,
    Variable,
    Binary,
    Unary,
    IfElse,
    Match,
    Loop,
    Block,
    Call,
    Tuple,
    Array,
    StructInit,
    Cast,
}

pub struct SelectionWeights;

impl SelectionWeights {
    pub fn pick_weighted<T: Copy, R: Rng>(choices: &[(T, f64)], rng: &mut R) -> T {
        let total_weight: f64 = choices.iter().map(|(_, w)| w).sum();
        if total_weight <= 0.0 {
            return choices[0].0;
        }
        let mut pick = rng.gen_range(0.0..total_weight);
        for (item, weight) in choices {
            if pick <= *weight {
                return *item;
            }
            pick -= *weight;
        }
        choices.last().unwrap().0
    }

    pub fn select_stmt_kind<R: Rng>(ctx: &Context, _policy: &FuzzPolicy, rng: &mut R) -> StmtKind {
        let mut choices = vec![
            (StmtKind::Let, 4.0),
            (StmtKind::Assign, 3.5),
            (StmtKind::Expr, 2.5),
        ];

        if ctx.return_expression_type.is_some() {
            choices.push((StmtKind::Return, 0.8));
        }
        if ctx.return_loop_type.is_some() {
            choices.push((StmtKind::Break, 0.8));
        }

        Self::pick_weighted(&choices, rng)
    }

    pub fn select_expr_category<R: Rng>(
        ctx: &Context,
        policy: &FuzzPolicy,
        rng: &mut R,
    ) -> ExprCategory {
        let depth = ctx.node_depth_stack.len();
        let max_depth = policy.max_depth;

        if depth >= max_depth {
            let leaf_choices = vec![(ExprCategory::Literal, 6.0), (ExprCategory::Variable, 4.0)];
            return Self::pick_weighted(&leaf_choices, rng);
        }

        let mut choices = vec![
            (ExprCategory::Literal, 3.5),
            (ExprCategory::Variable, 3.5),
            (ExprCategory::Binary, 4.5),
            (ExprCategory::Unary, 1.5),
            (ExprCategory::Cast, 1.5),
        ];

        let depth_penalty = 1.0 / (depth as f64 + 1.0);

        choices.push((ExprCategory::IfElse, 2.5 * depth_penalty));
        choices.push((ExprCategory::Block, 1.5 * depth_penalty));

        if policy.allow_matches {
            choices.push((ExprCategory::Match, 1.5 * depth_penalty));
        }
        if policy.allow_loops && depth < max_depth.saturating_sub(2) {
            choices.push((ExprCategory::Loop, 1.0 * depth_penalty));
        }
        if policy.allow_tuples {
            choices.push((ExprCategory::Tuple, 1.2 * depth_penalty));
        }
        if policy.allow_structs {
            choices.push((ExprCategory::StructInit, 1.2 * depth_penalty));
        }

        Self::pick_weighted(&choices, rng)
    }
}
