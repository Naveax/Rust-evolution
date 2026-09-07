enum __EvoEnum_Step {
    __EvoVariant_Small(i64),
    __EvoVariant_Large(i64),
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

fn __evo_fn_classify(__evo_value: i64) -> __EvoEnum_Step {
    if (__evo_value > 1000) {
        return __EvoEnum_Step::__EvoVariant_Large(__evo_value);
    } else {
        return __EvoEnum_Step::__EvoVariant_Small(__evo_value);
    }
}

fn main() {
    let __evo_n = __evo_input_int();
    let mut __evo_x = __evo_input_int();
    let mut __evo_sum = 0;
    for _ in 0..__evo_n {
        let __evo_step = __evo_fn_classify(__evo_x);
        match __evo_step {
            __EvoEnum_Step::__EvoVariant_Small(__evo_value) => {
                __evo_sum = (__evo_sum + __evo_value);
                __evo_x = (__evo_value + 7);
            },
            __EvoEnum_Step::__EvoVariant_Large(__evo_value) => {
                __evo_sum = (__evo_sum + __evo_value);
                __evo_x = (__evo_value / 2);
            },
        }
    }
    println!("{}", __evo_sum);
}
