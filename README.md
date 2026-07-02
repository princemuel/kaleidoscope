# Kaleidoscope 

An implementation of [llvm's C++ kaleidoscope language tutorial][tutorial] in Rust. 

Kaleidoscope is a small functional language,
built chapter by chapter alongside the original C++ reference.

The goal is not just to translate the C++ but to implement each stage
idiomatically in Rust: zero-copy lexing, sum-type AST, structured errors,
and safe(ish) LLVM bindings via [inkwell][inkwell], making use of Rust's inherent safety and expressiveness.

## Language overview

Kaleidoscope is a minimal expression language with:

- `f64` as the only numeric type
- User-defined functions (`def`)
- External function declarations (`extern`)
- Binary operators with user-definable precedence
- A REPL that parses and evaluates one line at a time

```console
ready> def foo(a b) a*a + 2*a*b + b*b;
Read function definition:
define double @foo(double %a, double %b) { ... }

ready> extern cos(x);
Read extern:
declare double @cos(double)

ready> cos(1.234);
Read top-level expression:
define double @__anon_expr() { ... }
```

## Chapters

### Stage 1: Lexer `(tag: stage1-lexer)`

**Tutorial chapter:** [Ch. 1 - Kaleidoscope Introduction and the Lexer][ch1]

The lexer transforms a `&str` into a stream of `TokenKind` values.
It operates on a pre-loaded string via a byte-index cursor.  no `getchar()`,
no global state.

Key implementation choices vs the C++ reference:

| C++                                | Rust                                         |
| ---------------------------------- | -------------------------------------------- |
| `getchar()` / `static LastChar`    | Byte-cursor over `&'a str`                   |
| `IdentifierStr` / `NumVal` globals | Data carried inside `TokenKind` variants     |
| Recursive comment handling         | Iterator `loop`                              |
| `strtod` accepts `"1.2.3"`         | Strict: digit-run then optional `.digit-run` |
| `".4"` parsed as `0.4`             | Matched. dot-led floats supported           |
| `"1."` parsed as a float           | Fixed. produces `Number(1.0)` + `Op('.')`   |

The lexer implements `Iterator<Item = Token<'a>>` (kind + span) and exposes
a `.tokens()` adaptor that strips spans for callers that don't need them.
`Comment` tokens are filtered transparently by the iterator; the parser never
sees them.

---

### Stage 2: Parser & AST `(tag: stage2-parser)`

**Tutorial chapter:** [Ch. 2.  Implementing a Parser and AST][ch2]

The parser is a hand-written recursive-descent parser with Pratt
(precedence-climbing) for binary expressions. It operates on a
pre-tokenised `&[TokenKind]` slice with a cursor index.

**AST:**

```rust
pub enum Expr {
    Number(f64),
    Variable(String),
    Binary { op: char, lhs: Box<Expr>, rhs: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

pub struct Function {
    pub proto: Prototype,
    pub body: Option<Expr>, // None = extern declaration
}
```

Using `body: Option<Expr>` on `Function` unifies `extern` declarations and
full definitions into one type, avoiding the separate `PrototypeAST` /
`FunctionAST` split from the C++ version.

**Error handling:**

All parse functions return `Result<T, ParseError>`. Errors carry the
offending token's text so messages are self-contained. The driver calls
`parser.skip_for_recovery()` on failure, matching C++'s `getNextToken()`
error-recovery pattern.

Two cursor advance variants make intent explicit at every call site:

- `advance()`.  hard fail: a token _must_ follow (e.g. after an operator)
- `advance_unchecked()`.  soft: EOF is grammatically acceptable (e.g. after `)`)

### Stage 3: LLVM IR Codegen `(tag: stage3-codegen)`

**Tutorial chapter:** [Ch. 3.  Code Generation to LLVM IR][ch3]

IR is emitted via [`inkwell`][inkwell], a safe Rust wrapper over the LLVM C API.
All LLVM objects share a lifetime `'ctx` tied to an owning `Context`:

```rust
pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub builder: Builder<'ctx>,
    pub module:  Module<'ctx>,
    pub symbols: HashMap<String, FloatValue<'ctx>>,
}
```

`Context` is owned by the caller (e.g. `main`), not by `Codegen`, avoiding
a self-referential struct.

**Expression codegen** (`Codegen::expr`):

| `Expr` variant         | LLVM output                                                 |
| ---------------------- | ----------------------------------------------------------- |
| `Number(f)`            | `ConstantFP`.  may constant-fold immediately                |
| `Variable(n)`          | Lookup in `symbols`; error if absent                        |
| `Binary { '+' }`       | `build_float_add` → `fadd`                                  |
| `Binary { '-' }`       | `build_float_sub` → `fsub`                                  |
| `Binary { '*' }`       | `build_float_mul` → `fmul`                                  |
| `Binary { '/' }`       | `build_float_div` → `fdiv`                                  |
| `Binary { '<' / '>' }` | `build_float_compare` (ULT) + `build_unsigned_int_to_float` |
| `Call { .. }`          | `build_call` after module symbol-table lookup               |

`>` is implemented as `<` with operands swapped, preserving consistent NaN
semantics via a single `ULT` predicate.

**Function codegen** (`Codegen::function`):

On success: creates an entry basic block, populates `symbols` from the
definition's prototype (not from the existing IR), emits the body, emits
`ret`, then calls `fn_val.verify(true)`.

On failure: calls `fn_val.delete()` so the user can redefine the function,
matching C++'s `eraseFromParent` intent.

**Bug fix over the C++ reference:**

The tutorial acknowledges a bug where an `extern` declaration takes
precedence over a later `def` with different parameter names:

```console
extern foo(a);   # ok
def foo(b) b;    # C++ error: Unknown variable name
```

This implementation fixes it by always populating `symbols` from
`proto.args` (the definition's prototype) rather than reading names back
from the existing IR, so `b` is correctly in scope regardless of what the
earlier `extern` declared.

## Prerequisites

| Requirement    | Version                   |
| -------------- | ------------------------- |
| Rust (nightly) | see `rust-toolchain.toml` |
| LLVM           | 22.x                      |
| `llvm-config`  | on `$PATH`                |

**Arch Linux:**

```sh
sudo pacman -S llvm
```

**macOS (Homebrew):**

```sh
brew install llvm@22
export PATH="$(brew --prefix llvm@22)/bin:$PATH"
```

**Ubuntu / Debian:**

```sh
wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh && sudo ./llvm.sh 22
```

Verify:

```sh
llvm-config --version   # should print 22.x.x
```

## Building

```sh
git clone https://github.com/princemuel/kaleidoscope
cd kaleidoscope
cargo build
```

Release build:

```sh
cargo build --release
```

## Running

```sh
cargo run
```

The REPL accepts one line at a time. Exit with `Ctrl+D` (Unix) or
`Ctrl+Z` + Enter (Windows). On exit, the full LLVM IR for the module is
printed to stderr.

**Example session:**

```console
ready> 4+5;
Read top-level expression:
define double @__anon_expr() {
entry:
  ret double 9.000000e+00
}

ready> def foo(a b) a*a + 2*a*b + b*b;
Read function definition:
define double @foo(double %a, double %b) {
entry:
  ...
}

ready> extern cos(x);
Read extern:
declare double @cos(double)

ready> cos(1.234);
Read top-level expression:
define double @__anon_expr() {
entry:
  %calltmp = call double @cos(double 1.234000e+00)
  ret double %calltmp
}
```

You can also pipe a source file:

```sh
cargo run < examples/fib.ks
```

## Testing

```sh
# All tests
cargo test

# A specific module
cargo test --lib -- lexer::tests
cargo test --lib -- parser::tests
cargo test --lib -- codegen::tests

# With stdout captured
cargo test -- --nocapture
```

The test suite covers:

- **Lexer**. Every token kind, span correctness, iterator fusing, clone
  independence, comment filtering, dot-led floats, number boundary cases
- **Parser**. Every parse method, all error variants, Pratt precedence
  and associativity, multi-dispatch (semicolon-separated statements),
  error recovery
- **Codegen**. Constant folding, all binary operators, symbol table
  scoping, extern→def resolution (the tutorial bug fix), redefinition
  rejection, failed-body cleanup

## Design notes

**Zero-copy lexing.** `TokenKind::Ident(&'a str)` borrows directly from
the source string. No identifier is ever copied into a `String` until the
parser owns it in an AST node.

**Two advance variants.** `advance()` and `advance_unchecked()` make the
grammar's EOF expectations explicit at every call site, rather than hiding
them behind a single fallible method.

**`body: Option<Expr>`.** Unifies `extern` declarations and function
definitions into one `Function` type. The `None` case is an extern; `Some`
is a definition.

**`#[non_exhaustive]` on `Expr`.** Prevents external crates from writing
exhaustive `match` arms. New variants (e.g. `If`, `For`) can be added in
later chapters without a breaking change.

**Structured errors.** `ParseError` and `CodegenError` are `thiserror`
enums with per-variant messages carrying the offending token or name.
The top-level `Error` type wraps both plus `io::Error`.

## Deviations from the C++ reference

| Area                             | C++                                                 | This implementation                                       |
| -------------------------------- | --------------------------------------------------- | --------------------------------------------------------- |
| Lexer input                      | `getchar()` / stdin, char-at-a-time                 | Pre-loaded `&str`, byte cursor                            |
| Token payload                    | Global `IdentifierStr`, `NumVal`                    | Carried inside `TokenKind` variants                       |
| Number lexing                    | Accepts `"1.2.3"` as a number                       | stricter grammar                                          |
| `".4"`                           | Parsed as `0.4`                                     | Matched                                                   |
| AST node dispatch                | `virtual codegen()` on each class                   | `match` on `Expr` in `Codegen::expr`                      |
| Extern / def types               | Separate `PrototypeAST` / `FunctionAST`             | Single `Function { body: Option<Expr> }`                  |
| Error reporting                  | `fprintf(stderr, ...)` + `nullptr` return           | `Result<T, ParseError>` / `Result<T, CodegenError>`       |
| `extern foo(a)` + `def foo(b) b` | **Bug: `b` not in scope**                           | **Fixed: symbols populated from definition**              |
| Anon function lifetime           | Kept in module (would conflict on 2nd) cter grammar |
| `".4"`                           | Parsed as `0.4`                                     | Matched                                                   |
| AST node dispatch                | `virtual codegen()` on each class                   | `match` on `Expr` in `Codegen::expr`                      |
| Extern / def types               | Separate `PrototypeAST` / `FunctionAST`             | Single `Function { body: Option<Expr> }`                  |
| Error reporting                  | `fprintf(stderr, ...)` + `nullptr` return           | `Result<T, ParseError>` / `Result<T, CodegenError>`       |
| `extern foo(a)` + `def foo(b) b` | **Bug: `b` not in scope**                           | **Fixed: symbols populated from definition**              |
| Anon function lifetime           | Kept in module (would conflict on 2nd)              | Deleted after printing (chapter 4 pattern, applied early) |

## License

Licensed under either of:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

[tutorial]: https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/index.html
[ch1]: https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl01.html
[ch2]: https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl02.html
[ch3]: https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl03.html
[inkwell]: https://github.com/TheDan64/inkwell
