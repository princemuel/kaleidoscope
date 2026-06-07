use std::collections::HashMap;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::FloatValue;

pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    named_values: HashMap<String, FloatValue<'ctx>>,
}

impl<'ctx> Codegen<'ctx> {
    #[must_use]
    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("my cool jit");
        let builder = context.create_builder();
        Self { context, module, builder, named_values: HashMap::new() }
    }
}
