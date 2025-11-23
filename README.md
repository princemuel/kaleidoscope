# Kaleidoscope

Kaleidoscope is a procedural language that allows you to define functions, use conditionals, math, etc.

This project is the reference implementation of “Kaleidoscope” Language tutorial ported to Rust.

The main tutorial shows how to implement a simple language using LLVM components in C++.

## Examples

```ruby
# Compute the x'th fibonacci number.
def fib(x)
  if x < 3 then
    1
  else
    fib(x-1)+fib(x-2)

# This expression will compute the 40th number.
fib(40)
```

```ruby
extern sin(arg);
extern cos(arg);
extern atan2(arg1 arg2);

atan2(sin(.4), cos(42))
```
