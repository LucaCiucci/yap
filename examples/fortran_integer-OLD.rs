use std::{ops::Range, path::PathBuf};

use yasp::basic::{Grammar, Text, Token};


trait FromToken: Sized {
    const GRAM: &str;
    fn from_token(src: &str, tok: &Token) -> anyhow::Result<Self>;
}

#[derive(Debug, Clone)]
pub struct SignedIntLiteralConstant {
    pub digits_span: Range<usize>,
    pub kind_param: Option<KindParam>,
}

impl FromToken for SignedIntLiteralConstant {
    const GRAM: &str = "signed-int-literal-constant";
    fn from_token(src: &str, tok: &Token) -> anyhow::Result<Self> {
        let digits_span = tok
            .iter_grams("digit-string")
            .map(|tok| tok.span.clone())
            .next()
            .ok_or(anyhow::anyhow!("No digits"))?;

        let kind_param = tok
            .iter_grams("kind-param")
            .map(|tok| KindParam::from_token(src, tok))
            .next()
            .transpose()?;

        Ok(SignedIntLiteralConstant { digits_span, kind_param })
    }
}

#[derive(Debug, Clone)]
pub enum KindParam {
    DigitString(Range<usize>),
    ScalarIntConstantName(Range<usize>),
}

impl FromToken for KindParam {
    const GRAM: &str = "kind-param";
    fn from_token(_src: &str, tok: &Token) -> anyhow::Result<Self> {
        if let Some(digits) = tok.iter_grams("digit-string").next() {
            return Ok(KindParam::DigitString(digits.span.clone()));
        }

        if let Some(name) = tok.iter_grams("scalar-int-constant-name").next() {
            return Ok(KindParam::ScalarIntConstantName(name.span.clone()));
        }

        Err(anyhow::anyhow!("No digits or name"))
    }
}

fn parse<G: FromToken>(src: &str) -> anyhow::Result<G> {
    let grammar: Grammar<Text> = serde_yaml::from_str(include_str!("fortran_integer.yaml"))
        .map_err(|e| anyhow::anyhow!("Failed to parse additional YAML: {}", e))?;

    let ebnf = format!("(* Generated from fortran_integer.yaml *)\n\n{}", grammar.to_ebnf(true));
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    std::fs::write(here.join("fortran_integer.ebnf"), ebnf)
        .map_err(|e| anyhow::anyhow!("Failed to write EBNF to file: {}", e))?;

    let (tok, diagnostics) = grammar.parse_non_term(G::GRAM, src)?
        .ok_or(anyhow::anyhow!("Failed to parse"))?;
    if !diagnostics.is_empty() {
        return Err(anyhow::anyhow!("Diagnostics: {:?}", diagnostics));
    }
    let result = G::from_token(src, &tok)?;
    Ok(result)
}

fn main() {
    let src = "1234567890_some_kind";
    match parse::<SignedIntLiteralConstant>(src) {
        Ok(lit) => {
            println!("Parsed: {:?}", lit);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}