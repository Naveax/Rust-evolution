struct __EvoRecord_Item {
    __evo_field_value: i64,
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

fn __evo_fn_read_value(__evo_item: &__EvoRecord_Item) -> i64 {
    return (__evo_item).__evo_field_value;
}

fn main() {
    let __evo_n = __evo_input_int();
    let __evo_seed = __evo_input_int();
    let mut __evo_item = __EvoRecord_Item { __evo_field_value: __evo_seed };
    let mut __evo_sum = 0;
    for _ in 0..__evo_n {
        __evo_sum = (__evo_sum + __evo_fn_read_value(&__evo_item));
        if ((__evo_item).__evo_field_value > 1000) {
            __evo_item = __EvoRecord_Item { __evo_field_value: ((__evo_item).__evo_field_value / 2) };
        } else {
            __evo_item = __EvoRecord_Item { __evo_field_value: ((__evo_item).__evo_field_value + 7) };
        }
    }
    println!("{}", __evo_sum);
}
