use std::path::PathBuf;

use yasp::basic::{Grammar, Text};

fn main() {
    // load grammar from YAML file
    let grammar: Grammar<Text> = serde_yaml::from_str(include_str!("fortran_integer.yaml"))
        .expect("Failed to deserialize grammar from YAML");

    // bonus: generate EBNF
    std::fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join("fortran_integer.ebnf"),
        format!(
            "(* Generated from fortran_integer.yaml *)\n\n{}",
            grammar.to_ebnf(true),
        ),
    ).expect("Failed to write EBNF to file");

    // sample input string
    let src = "1234567890_some_kind";

    // parse the input string using the grammar
    let (tok, diagnostics) = grammar.parse_non_term("signed-int-literal-constant", src)
        .expect("error while parsing")
        .expect("Failed to parse");

    if !diagnostics.is_empty() {
        eprintln!("Diagnostics:");
        for diag in diagnostics {
            eprintln!("  {}", diag.message());
        }
        panic!("Diagnostics is not empty");
    }

    if let Some(digits) = tok.iter_grams("digit-string").next() {
        let digits = &src[digits.span.clone()];
        println!("Digits: {}", digits);
    } else {
        panic!("No digits found");
    }

    if let Some(kind_param) = tok.iter_grams("kind-param").next() {
        let kind_param = &src[kind_param.span.clone()];
        println!("Kind param: {}", kind_param);
    } else {
        eprintln!("No kind param found");
    }
}