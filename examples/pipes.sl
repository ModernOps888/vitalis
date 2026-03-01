// Vitalis — Pipe Operator
// Tests the |> pipe operator for data flow

fn double(x: i64) -> i64 {
    x * 2
}

fn increment(x: i64) -> i64 {
    x + 1
}

fn square(x: i64) -> i64 {
    x * x
}

fn main() -> i64 {
    // Pipeline: 5 |> double |> increment
    let a: i64 = double(5);
    let b: i64 = increment(a);
    b
}
