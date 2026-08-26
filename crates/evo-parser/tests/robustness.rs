use evo_lexer::lex;
use evo_parser::{parse, parse_recovering};
use std::panic::{AssertUnwindSafe, catch_unwind};

const CASES_PER_SEED: usize = 128;
const RECOVERY_ERROR_CAP: usize = 8;
const MAX_SOURCE_BYTES: usize = 512;
const MUTATION_SEED: u64 = 0x4556_4f4c_5554_494f;

const SEEDS: &[&str] = &[
    "",
    "\n\n\n",
    "x = 1\nprint x\n",
    "repeat 2\nprint 1\nend\n",
    "repeat 2\nrepeat 1\nprint 1\nend\nend\n",
    "repeat 1\nprint 1\n",
    "repeat\nx = 1\nend\n",
    "x = 1 2\ny 2\n",
    ")\n)\n",
    "print \"hello\"\n",
    "print \"line\\nvalue\"\n",
    "print 1 + 2 * 3 / 4 - 5\n",
    "n = input_int\nrepeat n\nprint 1\nend\n",
    "end\nx 1\n",
    "print (1 + 2\n",
    "print 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15\n",
];

const INSERT_BYTES: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789_ \n\t=+-*/()#\"";
const FRAGMENTS: &[&str] = &[
    "print",
    "repeat",
    "end",
    "input_int",
    " = ",
    "\n",
    " 1",
    "\"x\"",
    "# comment\n",
    "(",
    ")",
    "+",
    "-",
    "*",
    "/",
];

#[derive(Clone, Copy)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }

    fn index(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next() % upper as u64) as usize
    }
}

#[test]
fn deterministic_mutation_sequence_is_stable() {
    let first = generated_sources().take(32).collect::<Vec<_>>();
    let second = generated_sources().take(32).collect::<Vec<_>>();
    assert_eq!(first, second);
}

#[test]
fn mutated_sources_preserve_parser_safety_and_api_consistency() {
    let mut lexable_cases = 0usize;
    let mut parser_error_cases = 0usize;

    for (seed_index, case_index, source) in generated_sources() {
        let Ok(tokens) = lex(&source) else {
            continue;
        };
        lexable_cases += 1;

        let context = case_context(seed_index, case_index, &source);
        let fail_fast = catch_unwind(AssertUnwindSafe(|| parse(&tokens)))
            .unwrap_or_else(|_| panic!("fail-fast parser panicked\n{context}"));
        let recovering = catch_unwind(AssertUnwindSafe(|| parse_recovering(&tokens)))
            .unwrap_or_else(|_| panic!("recovering parser panicked\n{context}"));

        match (fail_fast, recovering) {
            (Ok(expected), Ok(actual)) => {
                assert_eq!(
                    actual, expected,
                    "parser APIs disagree on valid input\n{context}"
                );
            }
            (Err(_), Err(errors)) => {
                parser_error_cases += 1;
                assert!(
                    !errors.is_empty(),
                    "recovery returned an empty error set\n{context}"
                );
                assert!(
                    errors.len() <= RECOVERY_ERROR_CAP,
                    "recovery exceeded the {RECOVERY_ERROR_CAP}-error cap: {} errors\n{context}",
                    errors.len()
                );
                for error in errors {
                    assert!(
                        error.span.start <= error.span.end,
                        "diagnostic span is reversed: {:?}\n{context}",
                        error.span
                    );
                    assert!(
                        error.span.end <= source.len(),
                        "diagnostic span escapes source bytes: {:?}, source_len={}\n{context}",
                        error.span,
                        source.len()
                    );
                    assert!(
                        error.span.line >= 1 && error.span.column >= 1,
                        "diagnostic line/column is not one-based: {:?}\n{context}",
                        error.span
                    );
                }
            }
            (Ok(program), Err(errors)) => panic!(
                "recovery rejected input accepted by fail-fast parser\nprogram={program:?}\nerrors={errors:?}\n{context}"
            ),
            (Err(error), Ok(program)) => panic!(
                "recovery silently accepted input rejected by fail-fast parser\nerror={error:?}\nprogram={program:?}\n{context}"
            ),
        }
    }

    assert!(
        lexable_cases >= 1_000,
        "robustness corpus produced only {lexable_cases} lexable parser cases"
    );
    assert!(
        parser_error_cases >= 500,
        "robustness corpus exercised only {parser_error_cases} parser-error cases"
    );
}

fn generated_sources() -> impl Iterator<Item = (usize, usize, String)> {
    SEEDS.iter().enumerate().flat_map(|(seed_index, seed)| {
        (0..CASES_PER_SEED).map(move |case_index| {
            let case_seed = MUTATION_SEED
                ^ (seed_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ (case_index as u64 + 1).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            let source = mutate(seed, case_seed);
            (seed_index, case_index, source)
        })
    })
}

fn mutate(seed: &str, case_seed: u64) -> String {
    let mut rng = Rng::new(case_seed);
    let mut bytes = seed.as_bytes().to_vec();
    let operations = 1 + rng.index(8);

    for _ in 0..operations {
        match rng.index(6) {
            0 => insert_byte(&mut bytes, &mut rng),
            1 => delete_byte(&mut bytes, &mut rng),
            2 => replace_byte(&mut bytes, &mut rng),
            3 => duplicate_slice(&mut bytes, &mut rng),
            4 => truncate(&mut bytes, &mut rng),
            5 => insert_fragment(&mut bytes, &mut rng),
            _ => unreachable!(),
        }
        if bytes.len() > MAX_SOURCE_BYTES {
            bytes.truncate(MAX_SOURCE_BYTES);
        }
    }

    String::from_utf8(bytes).expect("mutation alphabet is ASCII-only")
}

fn insert_byte(bytes: &mut Vec<u8>, rng: &mut Rng) {
    if bytes.len() >= MAX_SOURCE_BYTES {
        return;
    }
    let position = rng.index(bytes.len() + 1);
    let value = INSERT_BYTES[rng.index(INSERT_BYTES.len())];
    bytes.insert(position, value);
}

fn delete_byte(bytes: &mut Vec<u8>, rng: &mut Rng) {
    if bytes.is_empty() {
        insert_byte(bytes, rng);
        return;
    }
    let position = rng.index(bytes.len());
    bytes.remove(position);
}

fn replace_byte(bytes: &mut Vec<u8>, rng: &mut Rng) {
    if bytes.is_empty() {
        insert_byte(bytes, rng);
        return;
    }
    let position = rng.index(bytes.len());
    bytes[position] = INSERT_BYTES[rng.index(INSERT_BYTES.len())];
}

fn duplicate_slice(bytes: &mut Vec<u8>, rng: &mut Rng) {
    if bytes.is_empty() || bytes.len() >= MAX_SOURCE_BYTES {
        insert_byte(bytes, rng);
        return;
    }
    let start = rng.index(bytes.len());
    let remaining = bytes.len() - start;
    let length = 1 + rng.index(remaining.min(16));
    let duplicated = bytes[start..start + length].to_vec();
    let position = rng.index(bytes.len() + 1);
    bytes.splice(position..position, duplicated);
}

fn truncate(bytes: &mut Vec<u8>, rng: &mut Rng) {
    if bytes.is_empty() {
        return;
    }
    bytes.truncate(rng.index(bytes.len() + 1));
}

fn insert_fragment(bytes: &mut Vec<u8>, rng: &mut Rng) {
    if bytes.len() >= MAX_SOURCE_BYTES {
        return;
    }
    let fragment = FRAGMENTS[rng.index(FRAGMENTS.len())].as_bytes();
    let position = rng.index(bytes.len() + 1);
    bytes.splice(position..position, fragment.iter().copied());
}

fn case_context(seed_index: usize, case_index: usize, source: &str) -> String {
    format!(
        "mutation_seed=0x{MUTATION_SEED:016x}, seed_index={seed_index}, case_index={case_index}\nsource:\n{source:?}"
    )
}
