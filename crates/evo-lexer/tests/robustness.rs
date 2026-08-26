use evo_lexer::{LexError, Token, TokenKind, lex, lex_recovering};
use std::panic::{AssertUnwindSafe, catch_unwind};

const CASES_PER_SEED: usize = 128;
const RECOVERY_ERROR_CAP: usize = 8;
const MAX_SOURCE_BYTES: usize = 512;
const MUTATION_SEED: u64 = 0x4c45_5845_525f_465a;

const SEEDS: &[&str] = &[
    "",
    "\n\n\n",
    "x = 1\nprint x\n",
    "n = input_int\nrepeat n\nprint n\nend\n",
    "repeat 2\nrepeat 1\nprint 1\nend\nend\n",
    "# comment\nprint \"hello\"\n",
    "print \"line\\nvalue\"\n",
    "print \"bad\\q rest\"\nprint @\n",
    "print \"unterminated\nprint @\n",
    "print \"eof escape\\",
    "999999999999999999999999999999\nprint @\n",
    "@\n$\n%\n&\n!\n?\n~\n`\n^\n",
    "☃☃\n",
    "é = 1\nprint é\n",
    "#comment@ignored\nprint $\n",
    "print (1 + 2) * 3 / 4 - 5\n",
];

const INSERT_CHARS: &[char] = &[
    'a', 'z', '0', '9', '_', ' ', '\n', '\t', '=', '+', '-', '*', '/', '(', ')', '#', '"', '\\',
    '@', '$', '%', '☃', 'é',
];

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
    "@",
    "$",
    "☃",
    "é",
    "\"bad\\q\"",
    "\"unterminated\n",
    "999999999999999999999999999999",
    "\\",
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
    let first = generated_sources().take(64).collect::<Vec<_>>();
    let second = generated_sources().take(64).collect::<Vec<_>>();
    assert_eq!(first, second);
}

#[test]
fn mutation_corpus_preserves_lexer_safety_and_api_consistency() {
    let mut valid_cases = 0usize;
    let mut lexical_error_cases = 0usize;

    for (seed_index, case_index, source) in generated_sources() {
        let context = case_context(seed_index, case_index, &source);
        let fail_fast = catch_unwind(AssertUnwindSafe(|| lex(&source)))
            .unwrap_or_else(|_| panic!("fail-fast lexer panicked\n{context}"));
        let recovering = catch_unwind(AssertUnwindSafe(|| lex_recovering(&source)))
            .unwrap_or_else(|_| panic!("recovering lexer panicked\n{context}"));

        match (fail_fast, recovering) {
            (Ok(expected), Ok(actual)) => {
                valid_cases += 1;
                assert_eq!(
                    actual, expected,
                    "lexer APIs disagree on valid input\n{context}"
                );
                validate_tokens(&source, &actual, &context);
            }
            (Err(_), Err(errors)) => {
                lexical_error_cases += 1;
                validate_errors(&source, &errors, &context);
            }
            (Ok(tokens), Err(errors)) => panic!(
                "recovery rejected input accepted by fail-fast lexer\ntokens={tokens:?}\nerrors={errors:?}\n{context}"
            ),
            (Err(error), Ok(tokens)) => panic!(
                "recovery silently accepted input rejected by fail-fast lexer\nerror={error:?}\ntokens={tokens:?}\n{context}"
            ),
        }
    }

    assert!(
        valid_cases >= 200,
        "robustness corpus produced only {valid_cases} valid lexer cases"
    );
    assert!(
        lexical_error_cases >= 500,
        "robustness corpus exercised only {lexical_error_cases} lexical-error cases"
    );
}

#[test]
fn focused_recovery_regressions_remain_bounded_and_ordered() {
    let cases = [
        "print \"bad\\q rest\"\nprint @\n",
        "print \"unterminated\nprint @\n",
        "print \"eof escape\\",
        "999999999999999999999999999999\nprint @\n",
        "@\n$\n%\n&\n!\n?\n~\n`\n^\n",
        "☃☃\n",
        "# ignored @ $ ☃\nprint @\n",
    ];

    for source in cases {
        let errors = lex_recovering(source).expect_err("focused malformed input should fail");
        validate_errors(source, &errors, source);
    }
}

fn validate_tokens(source: &str, tokens: &[Token], context: &str) {
    assert!(
        !tokens.is_empty(),
        "valid lexing returned no tokens\n{context}"
    );
    assert_eq!(
        tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Eof))
            .count(),
        1,
        "valid token stream must contain exactly one EOF\n{context}"
    );
    let eof = tokens.last().expect("tokens are non-empty");
    assert!(
        matches!(eof.kind, TokenKind::Eof),
        "EOF must be last\n{context}"
    );
    assert_eq!(
        eof.span.start,
        source.len(),
        "EOF start mismatch\n{context}"
    );
    assert_eq!(eof.span.end, source.len(), "EOF end mismatch\n{context}");

    let mut previous_start = 0usize;
    for token in tokens {
        let span = token.span;
        assert!(
            span.start <= span.end,
            "token span reversed: {span:?}\n{context}"
        );
        assert!(
            span.end <= source.len(),
            "token span escapes source: {span:?}, source_len={}\n{context}",
            source.len()
        );
        assert!(
            source.is_char_boundary(span.start) && source.is_char_boundary(span.end),
            "token span is not on UTF-8 boundaries: {span:?}\n{context}"
        );
        assert!(
            span.start >= previous_start,
            "token spans move backwards: {span:?}\n{context}"
        );
        assert!(
            span.line >= 1 && span.column >= 1,
            "token line/column is not one-based: {span:?}\n{context}"
        );
        previous_start = span.start;
    }
}

fn validate_errors(source: &str, errors: &[LexError], context: &str) {
    assert!(
        !errors.is_empty(),
        "recovery returned empty errors\n{context}"
    );
    assert!(
        errors.len() <= RECOVERY_ERROR_CAP,
        "recovery exceeded {RECOVERY_ERROR_CAP}-error cap: {}\n{context}",
        errors.len()
    );

    let mut previous_start = 0usize;
    for error in errors {
        let span = error.span;
        assert!(
            span.start <= span.end,
            "error span reversed: {span:?}\n{context}"
        );
        assert!(
            span.end <= source.len(),
            "error span escapes source: {span:?}, source_len={}\n{context}",
            source.len()
        );
        assert!(
            source.is_char_boundary(span.start) && source.is_char_boundary(span.end),
            "error span is not on UTF-8 boundaries: {span:?}\n{context}"
        );
        assert!(
            span.start >= previous_start,
            "recovered errors are not source ordered: {span:?}\n{context}"
        );
        assert!(
            span.line >= 1 && span.column >= 1,
            "error line/column is not one-based: {span:?}\n{context}"
        );
        previous_start = span.start;
    }
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
    let mut chars = seed.chars().collect::<Vec<_>>();
    let operations = 1 + rng.index(8);

    for _ in 0..operations {
        match rng.index(6) {
            0 => insert_char(&mut chars, &mut rng),
            1 => delete_char(&mut chars, &mut rng),
            2 => replace_char(&mut chars, &mut rng),
            3 => duplicate_slice(&mut chars, &mut rng),
            4 => truncate(&mut chars, &mut rng),
            5 => insert_fragment(&mut chars, &mut rng),
            _ => unreachable!(),
        }
        truncate_to_max_bytes(&mut chars);
    }

    chars.into_iter().collect()
}

fn insert_char(chars: &mut Vec<char>, rng: &mut Rng) {
    let position = rng.index(chars.len() + 1);
    let value = INSERT_CHARS[rng.index(INSERT_CHARS.len())];
    chars.insert(position, value);
}

fn delete_char(chars: &mut Vec<char>, rng: &mut Rng) {
    if chars.is_empty() {
        insert_char(chars, rng);
        return;
    }
    let position = rng.index(chars.len());
    chars.remove(position);
}

fn replace_char(chars: &mut Vec<char>, rng: &mut Rng) {
    if chars.is_empty() {
        insert_char(chars, rng);
        return;
    }
    let position = rng.index(chars.len());
    chars[position] = INSERT_CHARS[rng.index(INSERT_CHARS.len())];
}

fn duplicate_slice(chars: &mut Vec<char>, rng: &mut Rng) {
    if chars.is_empty() {
        insert_char(chars, rng);
        return;
    }
    let start = rng.index(chars.len());
    let remaining = chars.len() - start;
    let length = 1 + rng.index(remaining.min(16));
    let duplicated = chars[start..start + length].to_vec();
    let position = rng.index(chars.len() + 1);
    chars.splice(position..position, duplicated);
}

fn truncate(chars: &mut Vec<char>, rng: &mut Rng) {
    if chars.is_empty() {
        return;
    }
    chars.truncate(rng.index(chars.len() + 1));
}

fn insert_fragment(chars: &mut Vec<char>, rng: &mut Rng) {
    let fragment = FRAGMENTS[rng.index(FRAGMENTS.len())].chars();
    let position = rng.index(chars.len() + 1);
    chars.splice(position..position, fragment);
}

fn truncate_to_max_bytes(chars: &mut Vec<char>) {
    let mut bytes = 0usize;
    let keep = chars
        .iter()
        .take_while(|ch| {
            let next = bytes + ch.len_utf8();
            if next > MAX_SOURCE_BYTES {
                false
            } else {
                bytes = next;
                true
            }
        })
        .count();
    chars.truncate(keep);
}

fn case_context(seed_index: usize, case_index: usize, source: &str) -> String {
    format!(
        "mutation_seed=0x{MUTATION_SEED:016x}, seed_index={seed_index}, case_index={case_index}\nsource:\n{source:?}"
    )
}
