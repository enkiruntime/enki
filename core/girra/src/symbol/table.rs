use super::ownership::{IdentifierData, OwnershipState};
use super::registry::GlobalRegistry;
use crate::types::Type;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, IdentifierData>>,
    pub registry: GlobalRegistry,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            registry: GlobalRegistry::new(),
        }
    }

    #[inline]
    pub fn current_depth(&self) -> usize {
        self.scopes.len().saturating_sub(1)
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, ty: Type, is_mutable: bool) {
        let name_str = name.into();
        let depth = self.current_depth();
        let data = IdentifierData::new(name_str.clone(), ty, is_mutable, depth);
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.insert(name_str, data);
        }
    }

    pub fn insert_const(&mut self, name: impl Into<String>, ty: Type) {
        let name_str = name.into();
        let data = IdentifierData::new_const(name_str.clone(), ty);
        if let Some(root_scope) = self.scopes.first_mut() {
            root_scope.insert(name_str, data);
        }
    }

    pub fn get(&self, name: &str) -> Option<&IdentifierData> {
        for scope in self.scopes.iter().rev() {
            if let Some(data) = scope.get(name) {
                return Some(data);
            }
        }
        None
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut IdentifierData> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(data) = scope.get_mut(name) {
                return Some(data);
            }
        }
        None
    }

    pub fn set_ownership(&mut self, name: &str, new_state: OwnershipState) -> bool {
        if let Some(var) = self.get_mut(name) {
            var.validity = new_state;
            true
        } else {
            false
        }
    }

    pub fn get_valid_variables(&self) -> Vec<&IdentifierData> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for scope in self.scopes.iter().rev() {
            for (name, data) in scope {
                if seen.insert(name) && data.validity == OwnershipState::Valid {
                    result.push(data);
                }
            }
        }
        result
    }

    pub fn get_assignable_variables(&self) -> Vec<&IdentifierData> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for scope in self.scopes.iter().rev() {
            for (name, data) in scope {
                if seen.insert(name)
                    && (data.is_mutable || data.ty.is_reference())
                    && data.validity.is_assignable()
                {
                    result.push(data);
                }
            }
        }
        result
    }

    pub fn find_variables_of_type<'a>(
        &'a self,
        target_type: &Type,
        mutable_required: bool,
    ) -> Vec<&'a IdentifierData> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for scope in self.scopes.iter().rev() {
            for (name, data) in scope {
                if seen.insert(name) && &data.ty == target_type {
                    let meets_mutability = if mutable_required {
                        data.is_mutable || matches!(data.ty, Type::Reference(ref r) if r.mutable)
                    } else {
                        true
                    };

                    let meets_ownership = if mutable_required {
                        data.validity.is_mutably_borrowable()
                    } else {
                        data.validity.is_borrowable() || data.validity.is_movable()
                    };

                    if meets_mutability && meets_ownership {
                        result.push(data);
                    }
                }
            }
        }
        result
    }

    pub fn find_containers_containing_type<'a>(
        &'a self,
        target_type: &Type,
    ) -> Vec<(&'a IdentifierData, &'a Type)> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for scope in self.scopes.iter().rev() {
            for (name, data) in scope {
                if seen.insert(name) && data.validity.is_borrowable() {
                    let members = data.ty.member_types();
                    if members.iter().any(|m| m == target_type) {
                        result.push((data, &data.ty));
                    }
                }
            }
        }
        result
    }
}
