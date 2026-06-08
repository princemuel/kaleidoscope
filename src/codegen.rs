use core::iter;
use std::collections::HashMap;

use inkwell::FloatPredicate;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{FloatValue, FunctionValue};

use crate::ast::{Expr, Function, Prototype};
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

    pub fn function(&mut self, func: &Function) -> Result<FunctionValue<'ctx>, CodegenError> {
        let proto = &func.proto;

        let fn_val = match self.module.get_function(&proto.name) {
            Some(existing) => {
                let actual = proto.args.len();
                #[expect(clippy::as_conversions)]
                let expected = existing.count_params() as usize;

                // Validate arity matches
                if expected != actual {
                    return Err(CodegenError::ArgCountMismatch { expected, got: actual });
                }
                // Reject if already has a body i.e redefinition
                if existing.count_basic_blocks() > 0 {
                    return Err(CodegenError::FunctionRedefinition(proto.name.clone()));
                }

                existing
            }
            None => self.proto(proto)?,
        };

        if fn_val.count_basic_blocks() > 0 {
            return Err(CodegenError::FunctionRedefinition(proto.name.clone()));
        }

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Record the function arguments in the SymbolsTable map.
        self.symbols.clear();
        for (param, name) in fn_val.get_param_iter().zip(proto.args.iter()) {
            param.into_float_value().set_name(name);
            self.symbols.insert(name.clone(), param.into_float_value());
        }

        let Some(body) = &func.body else { return Ok(fn_val) };

        // Clean up and propagate on either codegen failure or verification failure.
        let result = (|| -> Result<_, CodegenError> {
            let ret = self.expr(body)?;
            self.builder.build_return(Some(&ret))?;

            if !fn_val.verify(true) {
                return Err(CodegenError::VerificationFailed(proto.name.clone()));
            }

            Ok(fn_val)
        })();

        if result.is_err() {
            #[expect(unsafe_code)]
            unsafe {
                fn_val.delete();
            };
        }

        result
    }

    /// Compiles the specified `Prototype` into an extern LLVM `FunctionValue`.
    pub fn proto(&self, proto: &Prototype) -> Result<FunctionValue<'ctx>, CodegenError> {
        // Make the function type:  double(double,double) etc.
        let r#type = self.context.f64_type();
        let types: Vec<_> = iter::repeat_n(r#type, proto.args.len()).map(Into::into).collect();

        let fn_type = self.context.f64_type().fn_type(&types, false);
        let fn_val = self.module.add_function(proto.name.as_str(), fn_type, None);

        // // Set names for all arguments.
        // for (param, name) in fn_val.get_param_iter().zip(proto.args.iter()) {
        //     param.into_float_value().set_name(name);
        // }

        // finally return built prototype
        Ok(fn_val)
    }

    pub fn expr(&mut self, expr: &Expr) -> Result<FloatValue<'ctx>, CodegenError> {
        match expr {
            Expr::Number(n) => Ok(self.context.f64_type().const_float(*n)),

            Expr::Variable(name) => self
                .symbols
                .get(name)
                .copied()
                .ok_or_else(|| CodegenError::UnknownVariable(name.to_owned())),

            Expr::Binary { op, lhs, rhs } => {
                let op = *op;
                let lhs = self.expr(lhs)?;
                let rhs = self.expr(rhs)?;

                match op {
                    '+' => Ok(self.builder.build_float_add(lhs, rhs, "addtmp")?),
                    '-' => Ok(self.builder.build_float_sub(lhs, rhs, "subtmp")?),
                    '*' => Ok(self.builder.build_float_mul(lhs, rhs, "multmp")?),
                    '/' => Ok(self.builder.build_float_div(lhs, rhs, "divtmp")?),
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

                let actual = args.len();
                #[expect(clippy::as_conversions)]
                let expected = callee.count_params() as usize;

                if expected != actual {
                    return Err(CodegenError::ArgCountMismatch { expected, got: actual });
                }

                let compiled_args = args
                    .iter()
                    .map(|arg| self.expr(arg).map(Into::into))
                    .collect::<Result<Vec<_>, _>>()?;

                let call = self.builder.build_call(callee, &compiled_args, "calltmp")?;

                Ok(call
                    .try_as_basic_value()
                    .basic()
                    .ok_or_else(|| CodegenError::UnknownFunction(name.to_owned()))?
                    .into_float_value())
            }
        }
    }
}
