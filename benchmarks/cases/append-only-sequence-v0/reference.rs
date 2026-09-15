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
    let mut __evo_items = Vec::<i64>::new();
    let mut __evo_i = 0;
    for _ in 0..__evo_n {
        __evo_items.push(__evo_i);
        __evo_i = (__evo_i + 1);
    }
    let mut __evo_sum = 0;
    __evo_i = 0;
    for _ in 0..__evo_n {
        if let Some(&__evo_value) = usize::try_from(__evo_i).ok().and_then(|__evo_lookup_index| __evo_items.get(__evo_lookup_index)) {
            __evo_sum = (__evo_sum + __evo_value);
        } else {
            __evo_sum = (__evo_sum + 0);
        }
        __evo_i = (__evo_i + 1);
    }
    println!("{}", __evo_sum);
}
