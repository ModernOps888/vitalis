// Vitalis — Arithmetic test
// Tests binary operators: +, -, *, /, %

fn add(a: i64, b: i64) -> i64 {
    a + b
}

fn multiply(a: i64, b: i64) -> i64 {
    a * b
}

fn complex_math(x: i64) -> i64 {
    let a: i64 = x * 2;
    let b: i64 = a + 10;
    let c: i64 = b - 3;
    c
}

fn main() -> i64 {
    let result: i64 = add(20, 22);
    result
}
