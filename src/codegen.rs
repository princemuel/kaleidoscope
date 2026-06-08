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

        // no block or no builder setup needed for extern declarations
        let Some(body) = &func.body else { return Ok(fn_val) };

        let entry = self.context.append_basic_block(fn_val, "entry");
        self.builder.position_at_end(entry);

        // Record the function arguments in the SymbolsTable map.
        self.symbols.clear();
        for (param, name) in fn_val.get_param_iter().zip(proto.args.iter()) {
            param.into_float_value().set_name(name);
            self.symbols.insert(name.clone(), param.into_float_value());
        }

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

        self.symbols.clear();
        result
    }

    /// Compiles the specified `Prototype` into an extern LLVM `FunctionValue`.
    pub fn proto(&self, proto: &Prototype) -> Result<FunctionValue<'ctx>, CodegenError> {
        // Make the function type:  double(double,double) etc.
        let r#type = self.context.f64_type();
        let types: Vec<_> = iter::repeat_n(r#type, proto.args.len()).map(Into::into).collect();

        let fn_type = self.context.f64_type().fn_type(&types, false);
        let fn_val = self.module.add_function(proto.name.as_str(), fn_type, None);

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

#[cfg(test)]
mod tests {

    use core::assert_matches;

    use inkwell::context::Context;

    use crate::ast::{Expr, Function, Prototype};
    use crate::codegen::Codegen;
    use crate::error::CodegenError;

    /// Construct a fresh `Codegen` tied to the provided `Context`.
    /// Every test that needs a `Codegen` calls this so the context lifetime
    /// is owned by the test frame, not by the codegen struct.
    fn make_codegen(ctx: &Context) -> Codegen<'_> { Codegen::new("test_module", ctx) }

    /// Build a `Prototype` with the given name and argument names.
    fn proto(name: &str, args: &[&str]) -> Prototype {
        Prototype { name: name.to_owned(), args: args.iter().map(ToString::to_string).collect() }
    }

    /// Build a `Function` definition (with body).
    fn func(name: &str, args: &[&str], body: Expr) -> Function {
        Function { proto: proto(name, args), body: Some(body) }
    }

    /// Build an extern `Function` (no body).
    fn extern_func(name: &str, args: &[&str]) -> Function {
        Function { proto: proto(name, args), body: None }
    }

    /// `Expr::Number` shorthand for a float literal.
    fn num(v: f64) -> Expr { Expr::Number(v) }

    /// `Expr::Variable` shorthand.
    fn var(name: &str) -> Expr { Expr::Variable(name.to_owned()) }

    /// `Expr::Binary` shorthand.
    fn binop(op: char, lhs: Expr, rhs: Expr) -> Expr {
        Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) }
    }

    /// `Expr::Call` shorthand.
    fn call(name: &str, args: Vec<Expr>) -> Expr { Expr::Call { name: name.to_owned(), args } }

    #[test]
    fn expr_number_float() {
        use core::f64;

        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // Need a builder position — put it inside a scratch function.
        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let val = cg.expr(&num(f64::consts::PI)).unwrap();
        assert!(val.is_const());
        assert_eq!(val.get_constant(), Some((f64::consts::PI, false)));
    }

    #[test]
    fn expr_number_integer_represented_as_float() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let val = cg.expr(&Expr::Number(42.0)).unwrap();
        assert!(val.is_const());
        assert_eq!(val.get_constant(), Some((42.0, false)));
    }

    #[test]
    fn expr_variable_found_in_symbol_table() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        // Manually inject a constant into the symbol table to simulate a param.
        let sentinel = ctx.f64_type().const_float(7.0);
        cg.symbols.insert("x".to_owned(), sentinel);

        let val = cg.expr(&var("x")).unwrap();
        assert_eq!(val, sentinel);
    }

    #[test]
    fn expr_variable_unknown_returns_error() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let err = cg.expr(&var("undefined")).unwrap_err();
        assert_matches!(err, CodegenError::UnknownVariable(n) if n == "undefined");
    }

    /// Helper: compile a binary expression of two constants inside a scratch
    /// function and return the IR instruction name emitted.
    fn binary_instruction_name(op: char) -> String {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // Use a real function with params so operands are runtime values,
        // preventing constant folding.
        let double = ctx.f64_type();
        let ft = double.fn_type(&[double.into(), double.into()], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let a = f.get_nth_param(0).unwrap().into_float_value();
        let b = f.get_nth_param(1).unwrap().into_float_value();
        cg.symbols.insert("a".to_owned(), a);
        cg.symbols.insert("b".to_owned(), b);

        let expr = binop(op, var("a"), var("b"));
        let val = cg.expr(&expr).unwrap();
        val.get_name().to_str().unwrap().to_owned()
    }

    #[test]
    fn expr_binary_add_emits_fadd() {
        assert_eq!(binary_instruction_name('+'), "addtmp");
    }

    #[test]
    fn expr_binary_sub_emits_fsub() {
        assert_eq!(binary_instruction_name('-'), "subtmp");
    }

    #[test]
    fn expr_binary_mul_emits_fmul() {
        assert_eq!(binary_instruction_name('*'), "multmp");
    }

    #[test]
    fn expr_binary_div_emits_fdiv() {
        assert_eq!(binary_instruction_name('/'), "divtmp");
    }

    #[test]
    fn expr_binary_lt_emits_booltmp() {
        assert_eq!(binary_instruction_name('<'), "booltmp");
    }

    #[test]
    fn expr_binary_gt_emits_booltmp() {
        assert_eq!(binary_instruction_name('>'), "booltmp");
    }

    #[test]
    fn expr_binary_lt_constant_fold_true() {
        // 1.0 < 2.0 → 1.0
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let val = cg.expr(&binop('<', num(1.0), num(2.0))).unwrap();
        // IRBuilder constant-folds this to 1.0
        assert_eq!(val.get_constant(), Some((1.0, false)));
    }

    #[test]
    fn expr_binary_lt_constant_fold_false() {
        // 2.0 < 1.0 → 0.0
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let val = cg.expr(&binop('<', num(2.0), num(1.0))).unwrap();
        assert_eq!(val.get_constant(), Some((0.0, false)));
    }

    #[test]
    fn expr_binary_gt_is_symmetric_to_lt() {
        // `a > b` is `b < a`, so 2.0 > 1.0 → 1.0
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let val = cg.expr(&binop('>', num(2.0), num(1.0))).unwrap();
        assert_eq!(val.get_constant(), Some((1.0, false)));
    }

    #[test]
    fn expr_binary_unknown_op_returns_error() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let err = cg.expr(&binop('%', num(1.0), num(2.0))).unwrap_err();
        assert_matches!(err, CodegenError::InvalidBinaryOp('%'));
    }

    #[test]
    fn expr_call_unknown_function_returns_error() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let ft = ctx.f64_type().fn_type(&[], false);
        let f = cg.module.add_function("_scratch", ft, None);
        let bb = ctx.append_basic_block(f, "entry");
        cg.builder.position_at_end(bb);

        let err = cg.expr(&call("ghost", vec![])).unwrap_err();
        assert_matches!(err, CodegenError::UnknownFunction(n) if n == "ghost");
    }

    #[test]
    fn expr_call_arity_mismatch_returns_error() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // Declare a 2-arg function in the module.
        let double = ctx.f64_type();
        let ft = double.fn_type(&[double.into(), double.into()], false);
        cg.module.add_function("two_args", ft, None);

        // Position the builder.
        let scratch_ft = double.fn_type(&[], false);
        let scratch = cg.module.add_function("_scratch", scratch_ft, None);
        let bb = ctx.append_basic_block(scratch, "entry");
        cg.builder.position_at_end(bb);

        // Call with only one arg.
        let err = cg.expr(&call("two_args", vec![num(1.0)])).unwrap_err();
        assert_matches!(err, CodegenError::ArgCountMismatch { expected: 2, got: 1 });
    }

    #[test]
    fn expr_call_emits_calltmp() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // Declare `sin(x)` as a 1-arg extern.
        let double = ctx.f64_type();
        let ft = double.fn_type(&[double.into()], false);
        cg.module.add_function("sin", ft, None);

        let scratch_ft = double.fn_type(&[], false);
        let scratch = cg.module.add_function("_scratch", scratch_ft, None);
        let bb = ctx.append_basic_block(scratch, "entry");
        cg.builder.position_at_end(bb);

        let val = cg.expr(&call("sin", vec![num(1.0)])).unwrap();
        assert_eq!(val.get_name().to_str().unwrap(), "calltmp");
    }

    #[test]
    fn proto_creates_function_in_module() {
        let ctx = Context::create();
        let cg = make_codegen(&ctx);

        cg.proto(&proto("my_fn", &["a", "b"])).unwrap();
        assert!(cg.module.get_function("my_fn").is_some());
    }

    #[test]
    fn proto_correct_param_count() {
        let ctx = Context::create();
        let cg = make_codegen(&ctx);

        let fn_val = cg.proto(&proto("my_fn", &["x", "y", "z"])).unwrap();
        assert_eq!(fn_val.count_params(), 3);
    }

    #[test]
    fn proto_zero_args() {
        let ctx = Context::create();
        let cg = make_codegen(&ctx);

        let fn_val = cg.proto(&proto("nullary", &[])).unwrap();
        assert_eq!(fn_val.count_params(), 0);
    }

    #[test]
    fn proto_no_body_emitted() {
        // An extern prototype must have no basic blocks — it is a declaration only.
        let ctx = Context::create();
        let cg = make_codegen(&ctx);

        let fn_val = cg.proto(&proto("extern_fn", &["a"])).unwrap();
        assert_eq!(fn_val.count_basic_blocks(), 0);
    }

    #[test]
    fn proto_params_have_no_names() {
        // After our fix, proto() does not set param names — that is function()'s job.
        let ctx = Context::create();
        let cg = make_codegen(&ctx);

        let fn_val = cg.proto(&proto("unnamed", &["a", "b"])).unwrap();
        for param in fn_val.get_param_iter() {
            // LLVM represents an unnamed param as an empty string.
            assert_eq!(param.get_name().to_str().unwrap(), "");
        }
    }

    #[test]
    fn function_definition_compiles_and_verifies() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // def add(a b) a + b
        let body = binop('+', var("a"), var("b"));
        let fn_val = cg.function(&func("add", &["a", "b"], body)).unwrap();

        assert!(fn_val.verify(false));
        assert_eq!(fn_val.count_params(), 2);
        assert_eq!(fn_val.count_basic_blocks(), 1);
    }

    #[test]
    fn function_params_named_correctly() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let body = binop('+', var("x"), var("y"));
        let fn_val = cg.function(&func("named", &["x", "y"], body)).unwrap();

        let names: Vec<_> =
            fn_val.get_param_iter().map(|p| p.get_name().to_str().unwrap().to_owned()).collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn function_symbol_table_cleared_between_functions() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // First function with param `a`.
        cg.function(&func("first", &["a"], var("a"))).unwrap();
        assert!(
            !cg.symbols.contains_key("a"),
            "symbols must be cleared after function body is compiled"
        );
    }

    #[test]
    fn function_extern_then_def_resolves_correctly() {
        // This is the tutorial bug fix test.
        // extern foo(a);  →  def foo(b) b;  should succeed, not fail with
        // UnknownVariable.
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        cg.function(&extern_func("foo", &["a"])).unwrap();

        let result = cg.function(&func("foo", &["b"], var("b")));
        assert!(result.is_ok(), "def foo(b) b should resolve 'b', not 'a': {result:?}");
    }

    #[test]
    fn function_redefinition_returns_error() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        cg.function(&func("dup", &["x"], var("x"))).unwrap();

        let err = cg.function(&func("dup", &["x"], var("x"))).unwrap_err();
        assert_matches!(err, CodegenError::FunctionRedefinition(n) if n == "dup");
    }

    #[test]
    fn function_arity_mismatch_with_existing_extern_returns_error() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // Declare extern with 1 arg.
        cg.function(&extern_func("foo", &["a"])).unwrap();

        // Try to define with 2 args.
        let err =
            cg.function(&func("foo", &["a", "b"], binop('+', var("a"), var("b")))).unwrap_err();
        assert_matches!(err, CodegenError::ArgCountMismatch { expected: 1, got: 2 });
    }

    #[test]
    fn function_extern_has_no_body() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let fn_val = cg.function(&extern_func("cos", &["x"])).unwrap();
        assert_eq!(
            fn_val.count_basic_blocks(),
            0,
            "extern declaration must not emit a basic block"
        );
    }

    #[test]
    fn function_body_error_erases_from_module() {
        // If codegen of the body fails, the partial function must be removed
        // so the user can redefine it.
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        // Body references an undefined variable — will fail.
        let result = cg.function(&func("bad", &["x"], var("undefined")));
        assert!(result.is_err());

        // The function must have been erased.
        assert!(
            cg.module.get_function("bad").is_none(),
            "failed function must be erased from the module"
        );
    }

    #[test]
    fn function_recursive_call_compiles() {
        // def fib(x) fib(x)  — structurally valid even if it loops forever.
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let body = call("fib", vec![var("x")]);
        let result = cg.function(&func("fib", &["x"], body));
        assert!(result.is_ok());
    }

    // ── integration: full pipeline ────────────────────────────────────────────

    #[test]
    fn integration_constant_fold_addition() {
        // def anon() 4.0 + 5.0  →  ret double 9.0 (IRBuilder constant-folds)
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let body = binop('+', num(4.0), num(5.0));
        let fn_val = cg.function(&func("", &[], body)).unwrap();
        assert!(fn_val.verify(false));

        // The IR text must contain "ret double 9.0".
        let ir = fn_val.to_string();
        assert!(
            ir.contains("9.0") || ir.contains("9.000000e+00"),
            "expected constant-folded 9.0 in IR: {ir}"
        );
    }

    #[test]
    fn integration_call_extern() {
        // extern cos(x)
        // def anon() cos(1.234)
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        cg.function(&extern_func("cos", &["x"])).unwrap();

        let body = call("cos", vec![num(1.234)]);
        let fn_val = cg.function(&func("", &[], body)).unwrap();
        assert!(fn_val.verify(false));

        let ir = fn_val.to_string();
        assert!(ir.contains("@cos"), "expected call to @cos in IR: {ir}");
    }

    #[test]
    fn integration_quadratic_body() {
        // def foo(a b) a*a + 2*a*b + b*b
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        let body = binop(
            '+',
            binop(
                '+',
                binop('*', var("a"), var("a")),
                binop('*', binop('*', num(2.0), var("a")), var("b")),
            ),
            binop('*', var("b"), var("b")),
        );

        let fn_val = cg.function(&func("foo", &["a", "b"], body)).unwrap();
        assert!(fn_val.verify(false));

        let ir = fn_val.to_string();
        assert!(ir.contains("fmul"), "expected fmul instructions in IR: {ir}");
        assert!(ir.contains("fadd"), "expected fadd instructions in IR: {ir}");
    }

    #[test]
    fn integration_module_contains_all_functions() {
        let ctx = Context::create();
        let mut cg = make_codegen(&ctx);

        cg.function(&extern_func("sin", &["x"])).unwrap();
        cg.function(&func(
            "double_sin",
            &["x"],
            binop('+', call("sin", vec![var("x")]), call("sin", vec![var("x")])),
        ))
        .unwrap();

        assert!(cg.module.get_function("sin").is_some());
        assert!(cg.module.get_function("double_sin").is_some());
    }
}
