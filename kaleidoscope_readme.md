# 🎨 Kaleidoscope

A complete implementation of the LLVM Kaleidoscope tutorial language in idiomatic Rust.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## 📖 About

Kaleidoscope is a simple functional programming language designed for the [LLVM Tutorial](https://llvm.org/docs/tutorial/). This implementation follows the tutorial from beginning to end, but reimagines it in Rust with modern ergonomics, strong type safety, and zero-cost abstractions.

**What makes this different from the C++ tutorial:**
- 🦀 **Idiomatic Rust**: Leverages enums, pattern matching, and the type system for safer code
- 🔒 **Memory Safety**: No manual memory management or null pointers
- 🧪 **Comprehensive Testing**: Unit and integration tests for each component
- 📦 **Modular Design**: Clean separation between lexer, parser, codegen, and JIT
- ⚡ **Progressive Compilation**: Feature flags allow building without heavy LLVM dependencies

## ✨ Features

- [x] **Lexer & Parser** (Chapters 1-2)
  - Recursive descent parsing
  - Operator precedence climbing
  - Comprehensive error reporting

- [ ] **Code Generation** (Chapter 3)
  - LLVM IR generation
  - Expression compilation
  - Function definitions

- [ ] **JIT Compilation** (Chapter 4)
  - On-the-fly compilation and execution
  - Interactive REPL with immediate evaluation

- [ ] **Optimization** (Chapter 5)
  - LLVM optimization passes
  - Constant folding
  - Dead code elimination

- [ ] **Control Flow** (Chapter 6)
  - If/then/else conditionals
  - For loops with custom step

- [ ] **User-defined Operators** (Chapter 8)
  - Custom binary operators
  - Operator precedence specification

- [ ] **Mutable Variables** (Chapter 7)
  - Variable assignment
  - SSA form handling

- [ ] **Debug Information** (Chapter 9)
  - DWARF debug info generation
  - Source location tracking

## 🚀 Quick Start

### Prerequisites

```bash
# Rust (1.70 or later)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# LLVM 18 (for code generation features)
# macOS
brew install llvm@18

# Ubuntu/Debian
sudo apt-get install llvm-18-dev libpolly-18-dev

# Arch Linux
sudo pacman -S llvm
```

### Installation

```bash
git clone https://github.com/yourusername/kaleidoscope.git
cd kaleidoscope
cargo build --release
```

### Running the REPL

```bash
# Basic REPL (parser only, no LLVM required)
cargo run

# With JIT compilation (requires LLVM)
cargo run --features jit --release
```

## 💡 Usage

### Interactive REPL

```bash
$ cargo run --features jit
Kaleidoscope REPL
Type 'exit' to quit

ready> def fib(x)
         if x < 3 then
           1
         else
           fib(x-1) + fib(x-2);
Parsed a function definition.

ready> fib(10)
Evaluated to: 55.000000

ready> def binary : 1 (x y) 0;
ready> def binary > 10 (LHS RHS) RHS < LHS;
Parsed user-defined operators.

ready> 1 + 2 > 3
Evaluated to: 0.000000
```

### Example Programs

```kaleidoscope
# Mandelbrot set renderer
def binary : 1 (x y) y;

def printdensity(d)
  if d > 8 then
    putchard(32)
  else if d > 4 then
    putchard(46)
  else if d > 2 then
    putchard(43)
  else
    putchard(42);

# ... (see examples/mandelbrot.ks for full program)
```

## 🏗️ Project Structure

```
kaleidoscope/
├── src/
│   ├── lib.rs           # Library root
│   ├── main.rs          # REPL interface
│   ├── lexer.rs         # Tokenization
│   ├── parser.rs        # Recursive descent parser
│   ├── ast.rs           # Abstract Syntax Tree
│   ├── codegen/         # LLVM code generation
│   ├── jit.rs           # JIT compilation engine
│   ├── optimization/    # Optimization passes
│   └── error.rs         # Error handling
├── examples/            # Example Kaleidoscope programs
└── tests/               # Integration tests
```

## 🔧 Development

### Building Different Features

```bash
# Parser only (fast compilation, no LLVM)
cargo build

# With code generation
cargo build --features codegen

# Full featured (JIT + optimizations)
cargo build --features full

# Development with auto-reload
cargo watch -x "run --features jit"
```

### Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test lexer
cargo test parser

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test '*'
```

### Benchmarks

```bash
cargo bench
```

## 📚 Learning Resources

This implementation closely follows the [LLVM Kaleidoscope Tutorial](https://llvm.org/docs/tutorial/), but with Rust-specific adaptations:

- **Chapter 1**: [Kaleidoscope Introduction and Lexer](https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl01.html)
- **Chapter 2**: [Implementing a Parser and AST](https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl02.html)
- **Chapter 3**: [Code Generation to LLVM IR](https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl03.html)
- **Chapters 4-10**: JIT, Optimization, Control Flow, and more...

### Rust-Specific Concepts Used

- **Enums with Associated Data**: Token representation
- **Result Type**: Error handling without exceptions
- **Trait Objects**: Extensible AST nodes
- **Smart Pointers**: `Box<T>` for recursive types
- **Pattern Matching**: Parser dispatch and AST traversal
- **Generics**: Lexer works over any `Read` source

## 🤝 Contributing

Contributions are welcome! This project is primarily educational, so clear, well-documented code is prioritized over clever optimizations.

### Guidelines

1. Follow Rust idioms and conventions (`cargo fmt`, `cargo clippy`)
2. Add tests for new features
3. Update documentation for public APIs
4. Keep commits atomic and well-described

### Development Setup

```bash
# Install development tools
rustup component add rustfmt clippy

# Pre-commit checks
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [LLVM Project](https://llvm.org/) for the excellent tutorial
- [inkwell](https://github.com/TheDan64/inkwell) for safe Rust bindings to LLVM
- The Rust community for creating such an expressive language

## 🔗 Related Projects

- [LLVM Kaleidoscope Tutorial (C++)](https://llvm.org/docs/tutorial/)
- [Inkwell LLVM Bindings](https://github.com/TheDan64/inkwell)
- [Crafting Interpreters](https://craftinginterpreters.com/) - Another great language implementation book

---

**Status**: 🚧 Work in progress - Currently implementing Chapter 2 (Parser)

**Questions?** Open an issue or discussion on GitHub!
