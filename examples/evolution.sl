// Vitalis — Evolution Demo
// Showcases evolution-specific keywords and annotations

@evolvable
fn fitness_function(score: i64, complexity: i64) -> i64 {
    let raw: i64 = score * 100;
    let penalty: i64 = complexity * 5;
    raw - penalty
}

@evolvable
fn mutate_threshold(base: i64, generation: i64) -> i64 {
    if generation > 100 {
        base + 10
    } else {
        base
    }
}

fn main() -> i64 {
    let score: i64 = fitness_function(85, 12);
    let threshold: i64 = mutate_threshold(50, 150);
    score + threshold
}
