fn input_int() -> i64 {
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("failed to read integer input");
    input
        .trim()
        .parse::<i64>()
        .expect("expected signed integer input")
}

fn step(x: i64) -> i64 {
    if x > 1 && x != 7 {
        return x / 2;
    }
    x + 3
}

fn main() {
    let n = input_int();
    let mut x = input_int();
    let mut sum = 0;
    for _ in 0..n {
        x = step(x);
        sum += x;
    }
    println!("{}", sum);
}
