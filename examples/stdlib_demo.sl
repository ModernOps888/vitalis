// stdlib_demo.sl — Demonstrates the Vitalis Phase 1 Standard Library
//
// Capabilities showcased:
//   • Typed printing: i64, f64, bool, string literals
//   • Math: sqrt, pow, floor, ceil, abs, min, max, ln, sin, cos
//   • Type conversions: to_f64, to_i64
//   • String operations: str_len, str_eq, str_cat

@evolvable
fn demo_numerics() -> i64 {
    let x: i64 = 42
    println(x)                  // → 42
    let pi: f64 = 3.14159265
    println_f64(pi)             // → 3.14159265
    let flag: bool = true
    println_bool(flag)          // → true
    x
}

@evolvable
fn demo_strings() -> i64 {
    println_str("Vitalis 1.5 — Phase 1 stdlib active")
    let greeting: str = "Hello from "
    let world: str     = "Vitalis!"
    let combined: str  = str_cat(greeting, world)
    println_str(combined)       // → Hello from Vitalis!
    let length: i64 = str_len(greeting)
    println(length)             // → 11
    0
}

@evolvable
fn demo_math() -> f64 {
    let n: f64 = 144.0
    let root: f64 = sqrt(n)     // → 12.0
    println_f64(root)

    let base: f64 = 2.0
    let exp2: f64  = 10.0
    let pw: f64    = pow(base, exp2)  // → 1024.0
    println_f64(pw)

    let v: f64 = -7.5
    let av: f64 = abs_f64(v)    // → 7.5
    println_f64(av)

    let a: f64 = floor(3.9)     // → 3.0
    let b: f64 = ceil(3.1)      // → 4.0
    println_f64(a)
    println_f64(b)

    root
}

@evolvable
fn demo_transcendental() -> f64 {
    let e: f64 = 2.718281828
    let l: f64 = ln(e)          // → ~1.0
    println_f64(l)

    let pi_half: f64 = 1.5707963
    let s: f64 = sin(pi_half)   // → ~1.0
    println_f64(s)

    let c: f64 = cos(0.0)       // → 1.0
    println_f64(c)

    l
}

@evolvable
fn demo_conversions() -> f64 {
    let i: i64 = 7
    let f: f64 = to_f64(i)      // → 7.0
    println_f64(f)

    let fi: f64 = 3.99
    let ii: i64 = to_i64(fi)    // → 3 (truncate)
    println(ii)

    f
}

fn main() -> i64 {
    demo_numerics()
    demo_strings()
    demo_math()
    demo_transcendental()
    demo_conversions()
    0
}
