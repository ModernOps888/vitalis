// Vitalis — Structs and Types
// Tests struct definitions and field access

struct Point {
    x: i64,
    y: i64,
}

struct Rect {
    width: i64,
    height: i64,
}

fn area(r: Rect) -> i64 {
    r.width * r.height
}

fn main() -> i64 {
    let p: Point = Point { x: 10, y: 20 };
    let r: Rect = Rect { width: 5, height: 8 };
    p.x + p.y + area(r)
}
