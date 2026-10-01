use crate::types::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    // Statements
    Declaration,
    Assignment,
    ExpressionStatement,
    ReturnStatement,
    BreakStatement,

    // Expressions
    FunctionCall,
    MethodCall,
    Block,
    IfElse,
    If,
    Loop,
    Match,
    TupleAccess,
    StructAccess,
    VectorAccess,
    Reference,
    MutableReference,
    Dereference,
    RecursiveExpr,

    // Types
    StructType,
    TupleType,
    ArrayType,
    VectorType,
    BoxType,
    ReferenceType,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub node_depth_stack: Vec<HashMap<NodeKind, usize>>,
    pub required_type: Option<Type>,
    pub return_expression_type: Option<Type>,
    pub return_loop_type: Option<Type>,
    pub current_function_name: String,
    pub lifetime_requirement: Option<usize>,
    pub failed_nodes: Vec<NodeKind>,
    pub assignment_root_variable: Option<String>,
}

impl Context {
    pub fn new(function_name: impl Into<String>) -> Self {
        Self {
            node_depth_stack: vec![HashMap::new()],
            required_type: None,
            return_expression_type: None,
            return_loop_type: None,
            current_function_name: function_name.into(),
            lifetime_requirement: None,
            failed_nodes: Vec::new(),
            assignment_root_variable: None,
        }
    }

    pub fn enter_scope(&self) -> Self {
        let mut new_stack = self.node_depth_stack.clone();
        new_stack.push(HashMap::new());
        Self {
            node_depth_stack: new_stack,
            required_type: self.required_type.clone(),
            return_expression_type: self.return_expression_type.clone(),
            return_loop_type: self.return_loop_type.clone(),
            current_function_name: self.current_function_name.clone(),
            lifetime_requirement: self.lifetime_requirement,
            failed_nodes: Vec::new(),
            assignment_root_variable: self.assignment_root_variable.clone(),
        }
    }

    pub fn increment_depth(&self, kind: NodeKind) -> Self {
        let mut new_stack = self.node_depth_stack.clone();
        if let Some(top_scope) = new_stack.last_mut() {
            *top_scope.entry(kind).or_insert(0) += 1;
        }
        Self {
            node_depth_stack: new_stack,
            required_type: self.required_type.clone(),
            return_expression_type: self.return_expression_type.clone(),
            return_loop_type: self.return_loop_type.clone(),
            current_function_name: self.current_function_name.clone(),
            lifetime_requirement: self.lifetime_requirement,
            failed_nodes: self.failed_nodes.clone(),
            assignment_root_variable: self.assignment_root_variable.clone(),
        }
    }

    pub fn get_depth(&self, kind: NodeKind) -> usize {
        self.node_depth_stack
            .iter()
            .map(|scope| scope.get(&kind).copied().unwrap_or(0))
            .sum()
    }

    pub fn get_depth_local(&self, kind: NodeKind) -> usize {
        self.node_depth_stack
            .last()
            .and_then(|scope| scope.get(&kind).copied())
            .unwrap_or(0)
    }

    pub fn with_required_type(&self, ty: Type) -> Self {
        let mut clone = self.clone();
        clone.required_type = Some(ty);
        clone
    }

    pub fn with_return_type(&self, ty: Type) -> Self {
        let mut clone = self.clone();
        clone.return_expression_type = Some(ty);
        clone
    }

    pub fn with_loop_return_type(&self, ty: Type) -> Self {
        let mut clone = self.clone();
        clone.return_loop_type = Some(ty);
        clone
    }

    pub fn with_lifetime_requirement(&self, depth: usize) -> Self {
        let mut clone = self.clone();
        clone.lifetime_requirement = Some(depth);
        clone
    }

    pub fn with_assignment_target(&self, var_name: impl Into<String>) -> Self {
        let mut clone = self.clone();
        clone.assignment_root_variable = Some(var_name.into());
        clone
    }

    pub fn add_failed_node(&self, kind: NodeKind) -> Self {
        let mut clone = self.clone();
        if !clone.failed_nodes.contains(&kind) {
            clone.failed_nodes.push(kind);
        }
        clone
    }

    pub fn is_node_failed(&self, kind: NodeKind) -> bool {
        self.failed_nodes.contains(&kind)
    }
}
