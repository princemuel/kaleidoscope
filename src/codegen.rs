use std::collections::HashMap;

use inkwell::FloatPredicate;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::FloatValue;

use crate::ast::Expr;
use crate::error::CodegenError;

type SymbolTable<'ctx> = HashMap<String, FloatValue<'ctx>>;

pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    /// helper object that makes it easy to generate LLVM instructions
    pub builder: Builder<'ctx>,
    /// top-level structure the LLVM IR uses to contain code
    pub module: Module<'ctx>,
    /// the symbol table for the code aka `NamedValues`
    pub symbols: SymbolTable<'ctx>,
}

impl<'ctx> Codegen<'ctx> {
    #[must_use]
    pub fn new(name: &str, ctx: &'ctx Context) -> Self {
        let module = ctx.create_module(name);
        let builder = ctx.create_builder();
        Self { context: ctx, module, builder, symbols: SymbolTable::default() }
    }

    pub fn codegen_expr(&mut self, expr: &Expr) -> Result<FloatValue<'ctx>, CodegenError> {
        match expr {
            Expr::Number(n) => Ok(self.context.f64_type().const_float(*n)),

            Expr::Variable(name) => self
                .symbols
                .get(name)
                .copied()
                .ok_or_else(|| CodegenError::UnknownVariable(name.to_owned())),

            Expr::Binary { op, lhs, rhs } => {
                let op = *op;
                let lhs = self.codegen_expr(lhs)?;
                let rhs = self.codegen_expr(rhs)?;

                match op {
                    '+' => Ok(self.builder.build_float_add(lhs, rhs, "tmpadd")?),
                    '-' => Ok(self.builder.build_float_sub(lhs, rhs, "tmpsub")?),
                    '*' => Ok(self.builder.build_float_mul(lhs, rhs, "tmpmul")?),
                    '/' => Ok(self.builder.build_float_div(lhs, rhs, "tmpdiv")?),
                    '<' | '>' => {
                        let (lhs, rhs) = if op == '<' { (lhs, rhs) } else { (rhs, lhs) };

                        let cmp = self.builder.build_float_compare(
                            FloatPredicate::ULT,
                            lhs,
                            rhs,
                            "cmptmp",
                        )?;

                        Ok(self.builder.build_unsigned_int_to_float(
                            cmp,
                            self.context.f64_type(),
                            "booltmp",
                        )?)
                    }
                    _ => Err(CodegenError::InvalidBinaryOp(op)),
                }
            }

            Expr::Call { name, args } => {
                let callee = self
                    .module
                    .get_function(name)
                    .ok_or_else(|| CodegenError::UnknownFunction(name.to_owned()))?;

                #[expect(clippy::as_conversions)]
                let expected = callee.count_params() as usize;
                let actual = args.len();

                if expected != actual {
                    return Err(CodegenError::ArgCountMismatch { expected, got: actual });
                }

                let mut compiled_args = Vec::with_capacity(args.len());
                for arg in args {
                    compiled_args.push(self.codegen_expr(arg)?.into());
                }

                let call = self.builder.build_call(callee, &compiled_args, "tmpcall")?;

                Ok(call
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| CodegenError::UnknownFunction(name.to_owned()))?
                    .into_float_value())
            }
        }
    }
}
