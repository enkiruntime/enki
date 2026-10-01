use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct IdentGenerator {
    counters: HashMap<&'static str, usize>,
}

impl IdentGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_with_prefix(&mut self, prefix: &'static str) -> String {
        let count = self.counters.entry(prefix).or_insert(0);
        let name = format!("{}{}", prefix, count);
        *count += 1;
        name
    }

    pub fn next_var(&mut self) -> String {
        self.next_with_prefix("var_")
    }

    pub fn next_fn(&mut self) -> String {
        self.next_with_prefix("func_")
    }

    pub fn next_const(&mut self) -> String {
        self.next_with_prefix("CONST_")
    }

    pub fn next_struct(&mut self) -> String {
        self.next_with_prefix("Struct_")
    }

    pub fn next_type_alias(&mut self) -> String {
        self.next_with_prefix("Type_")
    }
}
