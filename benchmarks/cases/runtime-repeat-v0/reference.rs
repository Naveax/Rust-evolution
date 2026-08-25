fn __evo_input_int() -> i64 {
    let mut __evo_input = String::new();
    std::io::stdin()
        .read_line(&mut __evo_input)
        .expect("failed to read integer input");
    __evo_input
        .trim()
        .parse::<i64>()
        .expect("expected signed integer input")
}

fn main() {
    let __evo_n = __evo_input_int();
    let mut __evo_state = __evo_n;
    for _ in 0..__evo_n {
        __evo_state = ((__evo_state / 2) + 1);
    }
    println!("{}", __evo_state);
}
