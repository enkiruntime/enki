use crate::types::{ArrayType, StructType, TupleType, Type};
use rand::Rng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub name: String,
    pub args: Vec<(String, Type)>,
    pub return_type: Type,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalRegistry {
    pub structs: Vec<StructType>,
    pub tuples: Vec<TupleType>,
    pub arrays: Vec<ArrayType>,
    pub vectors: Vec<Type>,
    pub options: Vec<Type>,
    pub boxes: Vec<Type>,
    pub functions: Vec<FunctionSignature>,
}

impl GlobalRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_struct(&mut self, struct_type: StructType) {
        if !self.structs.iter().any(|s| s.name == struct_type.name) {
            self.structs.push(struct_type);
        }
    }

    pub fn get_random_struct<R: Rng>(&self, rng: &mut R) -> Option<StructType> {
        self.structs.choose(rng).cloned()
    }

    pub fn find_struct_containing_type(&self, target_type: &Type) -> Option<&StructType> {
        self.structs
            .iter()
            .find(|s| s.fields.iter().any(|f| &f.ty == target_type))
    }

    pub fn register_function(&mut self, signature: FunctionSignature) {
        if !self.functions.iter().any(|f| f.name == signature.name) {
            self.functions.push(signature);
        }
    }

    pub fn get_random_function_returning<R: Rng>(
        &self,
        return_type: &Type,
        rng: &mut R,
    ) -> Option<FunctionSignature> {
        let matching: Vec<&FunctionSignature> = self
            .functions
            .iter()
            .filter(|f| &f.return_type == return_type)
            .collect();
        matching.choose(rng).copied().cloned()
    }

    pub fn register_tuple(&mut self, tuple_type: TupleType) {
        if !self.tuples.contains(&tuple_type) {
            self.tuples.push(tuple_type);
        }
    }

    pub fn get_random_tuple<R: Rng>(&self, rng: &mut R) -> Option<TupleType> {
        self.tuples.choose(rng).cloned()
    }

    pub fn register_vector(&mut self, elem_type: Type) {
        if !self.vectors.contains(&elem_type) {
            self.vectors.push(elem_type);
        }
    }

    pub fn register_box(&mut self, elem_type: Type) {
        if !self.boxes.contains(&elem_type) {
            self.boxes.push(elem_type);
        }
    }

    pub fn register_option(&mut self, elem_type: Type) {
        if !self.options.contains(&elem_type) {
            self.options.push(elem_type);
        }
    }
}
