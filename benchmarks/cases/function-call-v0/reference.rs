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

fn __evo_fn_step(__evo_x: i64) -> i64 {
    if ((__evo_x > 1) && (!(__evo_x == 7))) {
        return (__evo_x / 2);
    } else {
        return (__evo_x + 3);
    }
}

fn main() {
    let __evo_n = __evo_input_int();
    let mut __evo_x = __evo_input_int();
    let mut __evo_sum = 0;
    for _ in 0..__evo_n {
        __evo_x = __evo_fn_step(__evo_x);
        __evo_sum = (__evo_sum + __evo_x);
    }
    println!("{}", __evo_sum);
}
