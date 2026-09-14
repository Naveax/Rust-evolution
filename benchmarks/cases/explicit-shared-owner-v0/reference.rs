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

fn __evo_fn_forward(__evo_item: std::rc::Rc<__EvoRecord_Item>) -> std::rc::Rc<__EvoRecord_Item> {
    return __evo_item;
}

fn main() {
    let __evo_n = __evo_input_int();
    let __evo_seed = __evo_input_int();
    let __evo_owner = std::rc::Rc::new(__EvoRecord_Item { __evo_field_value: __evo_seed });
    let mut __evo_sum = 0;
    for _ in 0..__evo_n {
        let __evo_alias = std::rc::Rc::clone(&(__evo_owner));
        __evo_sum = (__evo_sum + (__evo_alias).__evo_field_value);
        let __evo_moved = __evo_fn_forward(__evo_alias);
        __evo_sum = (__evo_sum + (__evo_moved).__evo_field_value);
    }
    println!("{}", __evo_sum);
}
