use crate::{basic::{Node, Text, Token}, gram};

use super::naive::Diagnostic;

pub fn cases() -> Vec<(Node<Text>, &'static str, Option<(Token, Vec<Diagnostic>)>)> {
    let mut tests: Vec<(Node<Text>, &str, Option<(Token, Vec<Diagnostic>)>)> = Vec::new();

    tests.push((
        gram! {
            ("f" | "foo" | "bar")+
        },
        "foo",
        Some((
            Token {
                span: 0..3,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![Token {
                    span: 0..3,
                    gram: None,
                    tags: vec![],
                    meta: Default::default(),
                    children: vec![],
                }],
            },
            vec![],
        )),
    ));

    tests.push((
        gram! {
            ("foo" | "bar")+
        },
        "barbar",
        Some((
            Token {
                span: 0..6,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![
                    Token {
                        span: 0..3,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                    Token {
                        span: 3..6,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                ],
            },
            vec![],
        )),
    ));

    tests.push((
        gram! {
            ("foo", ((" ", "bar")*))
        },
        "foo",
        Some((
            Token {
                span: 0..3,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![
                    Token {
                        span: 0..3,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                    Token {
                        span: 3..3,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                ],
            },
            vec![],
        )),
    ));

    tests.push((
        gram! {
            ("foo", ((" ", "bar")+))
        },
        "foo",
        Some((
            Token {
                span: 0..3,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![Token {
                    span: 0..3,
                    gram: None,
                    tags: vec![],
                    meta: Default::default(),
                    children: vec![],
                }],
            },
            vec![Diagnostic::Incomplete {
                span: 3..3,
                expected: gram!( (" ", "bar")+ ).to_string(),
            }],
        )),
    ));

    tests.push((
        gram! {
            ("foo", ((" ", "bar")?))
        },
        "foo",
        Some((
            Token {
                span: 0..3,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![
                    Token {
                        span: 0..3,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                    Token {
                        span: 3..3,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                ],
            },
            vec![],
        )),
    ));

    tests.push((
        gram! {
            ("foo", ((" ", "bar")*))
        },
        "foo bar bar",
        Some((
            Token {
                span: 0..11,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![
                    Token {
                        span: 0..3,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                    Token {
                        span: 3..11,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![
                            Token {
                                span: 3..7,
                                gram: None,
                                tags: vec![],
                                meta: Default::default(),
                                children: vec![
                                    Token {
                                        span: 3..4,
                                        gram: None,
                                        tags: vec![],
                                        meta: Default::default(),
                                        children: vec![],
                                    },
                                    Token {
                                        span: 4..7,
                                        gram: None,
                                        tags: vec![],
                                        meta: Default::default(),
                                        children: vec![],
                                    },
                                ],
                            },
                            Token {
                                span: 7..11,
                                gram: None,
                                tags: vec![],
                                meta: Default::default(),
                                children: vec![
                                    Token {
                                        span: 7..8,
                                        gram: None,
                                        tags: vec![],
                                        meta: Default::default(),
                                        children: vec![],
                                    },
                                    Token {
                                        span: 8..11,
                                        gram: None,
                                        tags: vec![],
                                        meta: Default::default(),
                                        children: vec![],
                                    },
                                ],
                            },
                        ],
                    },
                ],
            },
            vec![],
        )),
    ));

    tests.push((
        gram! {
            #r"[a-z]+"
        },
        "hello",
        Some((
            Token {
                span: 0..5,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![],
            },
            vec![],
        )),
    ));

    tests.push((
        gram! {
            ((#r"[a-z]+"), " ", "foo")
        },
        "hello foo",
        Some((
            Token {
                span: 0..9,
                gram: None,
                tags: vec![],
                meta: Default::default(),
                children: vec![
                    Token {
                        span: 0..5,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                    Token {
                        span: 5..6,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                    Token {
                        span: 6..9,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: vec![],
                    },
                ],
            },
            vec![],
        )),
    ));

    tests.push((
        gram! {
            ("foo" | "bar")+
        },
        "baz",
        None,
    ));

    tests
}
