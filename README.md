# Kaleidoscope

Kaleidoscope is a procedural language that allows you to define functions, use conditionals, math, etc.

This project is the reference implementation of [The “Kaleidoscope” Language tutorial][kaleidoscope] ported to Rust.

The main tutorial shows how to implement a simple language using LLVM components in C++.

[kaleidoscope]: https://llvm.org/docs/tutorial/MyFirstLanguageFrontend/LangImpl01.html#the-kaleidoscope-language

## Examples

```ks
# Compute the n'th fibonacci number.
def fib(n)
  if n < 3 then
    1
  else
    fib(n-1)+fib(n-2)

# This expression will compute the 40th number.
fib(40)
```

```ks
extern sin(arg);
extern cos(arg);
extern atan2(arg1 arg2);

atan2(sin(.4), cos(42))
```
