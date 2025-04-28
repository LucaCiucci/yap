
mod grammar;
mod node;
mod text;
mod token;

use std::fmt::Debug;

pub use grammar::*;
pub use node::*;
pub use text::*;
pub use token::*;

pub trait TerminalNode: Debug + PartialEq + Clone + 'static { // TODO loosen bounds
    type Src: ?Sized;
    fn parses(&self, src: &Self::Src, pos: usize) -> anyhow::Result<Option<usize>>;
    fn to_ebnf(&self) -> String;
}

mod serde_span_serialization {
    use std::ops::RangeInclusive;

    use bincode::{Decode, Encode};
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    #[derive(Serialize, Deserialize)]
    #[derive(Encode, Decode)]
    pub struct SerRange {
        #[serde(skip_serializing_if = "is_zero", default = "default_start")]
        min: usize,
        #[serde(skip_serializing_if = "is_max", default = "default_end")]
        max: usize
    }

    fn default_start() -> usize {
        0
    }

    fn default_end() -> usize {
        usize::MAX
    }

    impl From<RangeInclusive<usize>> for SerRange {
        fn from(range: RangeInclusive<usize>) -> Self {
            Self { min: *range.start(), max: *range.end() }
        }
    }
    
    impl From<SerRange> for RangeInclusive<usize> {
        fn from(ser_span: SerRange) -> Self {
            Self::new(ser_span.min, ser_span.max)
        }
    }

    fn is_zero(n: &usize) -> bool {
        *n == 0
    }

    fn is_max(n: &usize) -> bool {
        *n == usize::MAX
    }

    pub fn serialize<S>(span: &RangeInclusive<usize>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let ser_span = SerRange::from(span.clone());
        ser_span.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RangeInclusive<usize>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ser_span = SerRange::deserialize(deserializer)?;
        Ok(RangeInclusive::from(ser_span))
    }
}

#[macro_export]
macro_rules! gram {
    ($($any:tt)*) => {
        $crate::generic_gram! { $crate::basic::Text => $($any)+ }
    };
}

#[macro_export]
macro_rules! generic_gram {
    ($T:ty => $any:tt+) => {
        $crate::basic::Node::<$T>::rep($crate::generic_gram!($T => ::unwrap $any), 1..)
    };
    ($T:ty => $any:tt*) => {
        $crate::basic::Node::<$T>::rep($crate::generic_gram!($T => ::unwrap $any), 0..)
    };
    ($T:ty => $any:tt?) => {
        $crate::basic::Node::<$T>::rep($crate::generic_gram!($T => ::unwrap $any), 0..=1)
    };
    ($T:ty => $non_term:ident) => {
        $crate::basic::Node::<$T>::NonTerm(stringify!($non_term).to_string())
    };
    ($T:ty => $term:literal) => {
        $crate::basic::Node::<$T>::Terminal($crate::basic::Text::String($term.into()))
    };
    ($T:ty => #$regex:literal) => {
        $crate::basic::Node::<$T>::Terminal($crate::basic::Text::Regex($regex.into()))
    };
    ($T:ty => ($($any:tt),+)) => {
        $crate::basic::Node::<$T>::Seq(vec![$($crate::generic_gram!($T => ::unwrap $any)),+])
    };
    ($T:ty => ($($any:tt)|+)) => {
        $crate::basic::Node::<$T>::Alt(vec![$($crate::generic_gram!($T => ::unwrap $any)),+])
    };
    ($T:ty => $tag:literal : $($tail:tt)+) => {
        $crate::basic::Node::<$T>::Tagged { node: Box::new($crate::generic_gram!($T => $($tail)+)), tag: $tag.into() }
    };


    ($T:ty => ::unwrap ($any:tt+)) => { $crate::generic_gram!($T => $any+) };
    ($T:ty => ::unwrap ($any:tt*)) => { $crate::generic_gram!($T => $any*) };
    ($T:ty => ::unwrap ($any:tt?)) => { $crate::generic_gram!($T => $any?) };
    ($T:ty => ::unwrap $non_term:ident) => { $crate::generic_gram!($T => $non_term) };
    ($T:ty => ::unwrap $term:literal) => { $crate::generic_gram!($T => $term) };
    ($T:ty => ::unwrap (#$regex:literal)) => { $crate::generic_gram!($T => #$regex) };
    ($T:ty => ::unwrap ($($any:tt),+)) => { $crate::generic_gram!($T => ($($any),+)) };
    ($T:ty => ::unwrap ($($any:tt)|+)) => { $crate::generic_gram!($T => ($($any)|+)) };
    ($T:ty => ::unwrap ($tag:literal : $($tail:tt)+)) => { $crate::generic_gram!($T => $tag : $($tail)*) };
    ($T:ty => ::unwrap $any:tt) => { $crate::generic_gram!($T => $any) };
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro() {
        assert_eq!(
            gram!(a+),
            Node::rep(
                Node::<Text>::NonTerm("a".to_string()),
                1..,
            ),
        );
        assert_eq!(
            gram!(a*),
            Node::rep(
                Node::<Text>::NonTerm("a".to_string()),
                0..,
            ),
        );
        assert_eq!(
            gram!(a?),
            Node::rep(
                Node::<Text>::NonTerm("a".to_string()),
                0..2,
            ),
        );
        assert_eq!(
            gram!(a),
            Node::<Text>::NonTerm("a".to_string()),
        );
        assert_eq!(
            gram!("a"),
            Node::Terminal(Text::String("a".to_string())),
        );
        assert_eq!(
            gram!(#r"a"),
            Node::Terminal(Text::Regex("a".to_string())),
        );
        assert_eq!(
            gram!('a'),
            Node::Terminal(Text::String("a".to_string())),
        );
        assert_eq!(
            gram!(#r"^a$"),
            Node::Terminal(Text::Regex("^a$".to_string())),
        );
        assert_eq!(
            gram!((a)),
            Node::Seq(vec![
                Node::NonTerm("a".to_string()),
            ]),
        );
        assert_eq!(
            gram!((a, b)),
            Node::Seq(vec![
                Node::NonTerm("a".to_string()),
                Node::NonTerm("b".to_string()),
            ]),
        );
        assert_eq!(
            gram!((a | b)),
            Node::Alt(vec![
                Node::NonTerm("a".to_string()),
                Node::NonTerm("b".to_string()),
            ]),
        );
        assert_eq!(
            gram!("tag":a),
            Node::tagged(
                Node::NonTerm("a".to_string()),
                "tag".to_string(),
            ),
        );

        assert_eq!(
            gram!((a, (b*), ("tag":a))),
            Node::Seq(vec![
                Node::NonTerm("a".to_string()),
                Node::rep(
                    Node::NonTerm("b".to_string()),
                    0..,
                ),
                Node::tagged(
                    Node::NonTerm("a".to_string()),
                    "tag".to_string(),
                ),
            ]),
        );
    }

    #[test]
    fn test_ebnf() {
        assert_eq!(gram!(a+).to_ebnf(), "a+");
        assert_eq!(gram!(a*).to_ebnf(), "a*");
        assert_eq!(gram!(a?).to_ebnf(), "[a]");
        assert_eq!(gram!((a, b)).to_ebnf(), "a b");
        assert_eq!(gram!((a | b)).to_ebnf(), "a | b");
        assert_eq!(gram!("a").to_ebnf(), "\"a\"");
        assert_eq!(gram!(#r"a").to_ebnf(), "/a/");
        assert_eq!(gram!(a).to_ebnf(), "a");
    }
}

#[cfg(test)]
mod ebnf_tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn test_load_ebnf_simple() {
        let source = r#"
            digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
        "#;
        let grammar = Grammar::load_ebnf(source).expect("Failed to load EBNF");
        assert_eq!(
            grammar.rules,
            [
                (
                    "digit".to_string(),
                    gram!(("0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")),
                ),
            ].into_iter().collect::<BTreeMap<_, _>>()
        );
    }

    #[test]
    fn test_load_ebnf_repetition() {
        let source = r#"
            digits = digit+;
            digit = "0" | "1";
        "#;
        let grammar = Grammar::load_ebnf(source).expect("Failed to load EBNF");
        assert_eq!(
            grammar.rules,
            [
                (
                    "digits".to_string(),
                    gram!(digit+),
                ),
                (
                    "digit".to_string(),
                    gram!(("0" | "1")),
                ),
            ].into_iter().collect::<BTreeMap<_, _>>()
        );
    }

    #[test]
    fn test_load_ebnf_optional() {
        let source = r#"
            optional_digit = digit?;
            digit = "0";
        "#;
        let grammar = Grammar::load_ebnf(source).expect("Failed to load EBNF");
        assert_eq!(
            grammar.rules,
            [
                (
                    "optional_digit".to_string(),
                    gram!(digit?),
                ),
                (
                    "digit".to_string(),
                    gram!("0"),
                ),
            ].into_iter().collect::<BTreeMap<_, _>>()
        );
    }

    #[test]
    fn test_load_ebnf_sequence() {
        let source = r#"
            sequence = "a" , "b" , "c";
        "#;
        let grammar = Grammar::load_ebnf(source).expect("Failed to load EBNF");
        assert_eq!(
            grammar.rules,
            [
                (
                    "sequence".to_string(),
                    gram!(("a", "b", "c")),
                ),
            ].into_iter().collect::<BTreeMap<_, _>>()
        );
    }

    #[test]
    fn test_load_ebnf_grouping() {
        let source = r#"
            grouped = ("a" | "b") , "c";
        "#;
        let grammar = Grammar::load_ebnf(source).expect("Failed to load EBNF");
        assert_eq!(
            grammar.rules,
            [
                (
                    "grouped".to_string(),
                    gram!((("a" | "b"), "c")),
                ),
            ].into_iter().collect::<BTreeMap<_, _>>()
        );
    }

    #[test]
    fn test_load_ebnf_complex() {
        let source = r#"
            expression = term , (("+" | "-") , term)*;
            term = factor , (("*" | "/") , factor)*;
            factor = ("(" , expression , ")") | number;
            number = digit+;
            digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
        "#;
        let grammar = Grammar::load_ebnf(source).expect("Failed to load EBNF");
        assert_eq!(
            grammar.rules,
            [
                (
                    "expression".to_string(),
                    gram!( (term, ((("+" | "-"), term)*)) ),
                ),
                (
                    "term".to_string(),
                    gram!( (factor, ((("*" | "/"), factor)*)) ),
                ),
                (
                    "factor".to_string(),
                    gram!((("(", expression, ")") | number)),
                ),
                (
                    "number".to_string(),
                    gram!(digit+),
                ),
                (
                    "digit".to_string(),
                    gram!(("0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")),
                ),
            ].into_iter().collect::<BTreeMap<_, _>>()
        );
    }

    #[test]
    fn iter_label() {
        let token = Token {
            span: 0..0,
            gram: Some("digit".to_string()),
            tags: vec!["label-1".to_string()],
            meta: Default::default(),
            children: vec![
                Token {
                    span: 0..1,
                    gram: None,
                    tags: vec!["label-2".to_string()],
                    meta: Default::default(),
                    children: vec![
                        Token {
                            span: 0..3,
                            gram: None,
                            tags: vec!["label-2".to_string(), "label-3".to_string()],
                            meta: Default::default(),
                            children: vec![],
                        },
                    ],
                },
                Token {
                    span: 0..2,
                    gram: None,
                    tags: vec!["label-3".to_string()],
                    meta: Default::default(),
                    children: vec![],
                },
            ],
        };

        let label_1 = token.iter_label("label-1").map(|t| t.span.end).collect::<Vec<_>>();
        assert_eq!(label_1, vec![0]);
        let label_2 = token.iter_label("label-2").map(|t| t.span.end).collect::<Vec<_>>();
        assert_eq!(label_2, vec![1, 3]);
        let label_3 = token.iter_label("label-3").map(|t| t.span.end).collect::<Vec<_>>();
        assert_eq!(label_3, vec![2, 3]);
    }
}