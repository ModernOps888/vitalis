// Vitalis — Control Flow
// Tests if/else, match expressions

fn abs(x: i64) -> i64 {
    if x > 0 {
        x
    } else {
        0 - x
    }
}

fn classify(n: i64) -> i64 {
    if n > 100 {
        3
    } else {
        if n > 10 {
            2
        } else {
            1
        }
    }
}

fn main() -> i64 {
    let a: i64 = abs(0 - 42);
    let b: i64 = classify(50);
    a + b
}
