struct __EvoRecord_Pair {
    __evo_field_left: i64,
    __evo_field_right: i64,
}

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
    let mut __evo_x = __evo_input_int();
    let mut __evo_sum = 0;
    for _ in 0..__evo_n {
        let __evo_pair = __EvoRecord_Pair { __evo_field_left: __evo_x, __evo_field_right: 7 };
        __evo_sum = ((__evo_sum + (__evo_pair).__evo_field_left) + (__evo_pair).__evo_field_right);
        if ((__evo_pair).__evo_field_left > (__evo_pair).__evo_field_right) {
            __evo_x = ((__evo_pair).__evo_field_left / 2);
        } else {
            __evo_x = ((__evo_pair).__evo_field_left + (__evo_pair).__evo_field_right);
        }
    }
    println!("{}", __evo_sum);
}
