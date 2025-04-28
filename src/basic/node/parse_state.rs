use std::{collections::{BTreeMap, HashMap}, ops::RangeInclusive};

use crate::{basic::{Grammar, Node, TerminalNode, Token}, parsers::naive::{AbstractStackState, Diagnostic, Parsed, StackPoll}};

#[derive(Debug, Clone)]
pub struct State<'a, T: TerminalNode> {
    pub(super) grammar: &'a Grammar<T>,
    pub(super) cache: HashMap<(String, usize), Option<Parsed<&'a Node<T>>>>
}

impl<'a, T: TerminalNode> State<'a, T> {
    pub fn new(grammar: &'a Grammar<T>) -> Self {
        Self {
            grammar,
            cache: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum StackState<'a, T: TerminalNode> {
    ParsingSequence {
        elements: &'a[Node<T>],
        parsed: Vec<Token>,
        diagnostics: Vec<Diagnostic>,
    },
    ParsingChoice {
        start_pos: usize,
        elements: &'a[Node<T>],
        current: usize,
        parsed: Vec<(Parsed<&'a Node<T>>, usize)>,
    },
    ParsingRepetition {
        start_pos: usize,
        element: &'a Node<T>,
        range: RangeInclusive<usize>,
        parsed: Vec<Token>,
        diagnostics: Vec<Diagnostic>,
    },
    ParsingNonTerminal {
        start_pos: usize,
        name: &'a str,
    },
    ParsingTagged {
        tag: String,
    },
    ParsingMeta {
        meta: BTreeMap<String, String>,
    },
}

impl<'a, T: TerminalNode + 'static> StackState<'a, T> {
    fn poll_choice(
        next: Option<Parsed<&'a Node<T>>>,
        start_pos: usize,
        elements: &'a [Node<T>],
        mut current: usize,
        mut parsed: Vec<(Parsed<&'a Node<T>>, usize)>,
    ) -> StackPoll<&'a Node<T>> {
        assert_ne!(elements.len(), 0, "Empty choice");
        parsed.extend(next.map(|p| (p, current)));
        current += 1;
        if current >= elements.len() {
            if parsed.is_empty() {
                StackPoll::Finished(None)
            } else {
                // pick the longest one
                // TODO avoid sorting
                parsed.sort_by_key(|(p, _)| (p.token.span.end, p.incomplete.is_none()));
                StackPoll::Finished(parsed.pop().map(|p| p.0))
            }
        } else {
            StackPoll::Feed(
                Self::ParsingChoice {
                    start_pos,
                    elements,
                    current,
                    parsed,
                },
                &elements[current],
                start_pos,
            )
        }
    }

    fn poll_repetition(
        next: Option<Parsed<&'a Node<T>>>,
        element: &'a Node<T>,
        range: RangeInclusive<usize>,
        mut parsed: Vec<Token>,
        start_pos: usize,
        mut diagnostics: Vec<Diagnostic>,
    ) -> StackPoll<&'a Node<T>> {
        // FIXME this is not idiomatic
        let next = if let Some(next) = next {
            if next.token.span.start >= next.token.span.end {
                None
            } else {
                Some(next)
            }
        } else {
            None
        };

        if let Some(Parsed { token, diagnostics: sub_diag, incomplete }) = next {
            parsed.push(token);
            diagnostics.extend(sub_diag);
            if parsed.len() >= *range.end() {
                let start = parsed.first().map(|f| f.span.start).unwrap_or(start_pos);
                let end = parsed.last().map(|f| f.span.end).unwrap_or(start_pos);
                StackPoll::Finished(Some(Parsed {
                    token: Token {
                        span: start..end,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: parsed,
                    },
                    diagnostics,
                    incomplete,
                }))
            } else {
                let end = parsed.last().unwrap().span.end;
                StackPoll::Feed(
                    Self::ParsingRepetition {
                        start_pos,
                        element,
                        range,
                        parsed,
                        diagnostics,
                    },
                    element,
                    end,
                )
            }
        } else if parsed.len() == 0 && *range.start() > 0 {
            StackPoll::Finished(None)
        } else if parsed.len() < *range.start() {
            let start = parsed.first().map(|f| f.span.start).unwrap_or(start_pos);
            let end = parsed.last().map(|f| f.span.end).unwrap_or(start_pos);
            // TODO more specific error
            diagnostics.push(Diagnostic::Incomplete {
                span: end..end,
                expected: element.to_ebnf(),
            });
            StackPoll::Finished(Some(Parsed {
                token: Token {
                    span: start..end,
                    gram: None,
                    tags: vec![],
                    meta: Default::default(),
                    children: parsed,
                },
                diagnostics,
                incomplete: Some(element),
            }))
        } else {
            let start = parsed.first().map(|f| f.span.start).unwrap_or(start_pos);
            let end = parsed.last().map(|f| f.span.end).unwrap_or(start_pos);
            StackPoll::Finished(Some(Parsed {
                token: Token {
                    span: start..end,
                    gram: None,
                    meta: Default::default(),
                    tags: vec![],
                    children: parsed,
                },
                diagnostics, // TODO!!!!
                incomplete: None,
            }))
        }
    }

    fn poll_non_terminal(
        next: Option<Parsed<&'a Node<T>>>,
        name: &'a str,
        start_pos: usize,
        state: &mut State<'a, T>,
    ) -> StackPoll<&'a Node<T>> {
        let cache_key = (name.to_string(), start_pos);
        let parsed = if let Some(Parsed { token, diagnostics, incomplete }) = next {
            let start = token.span.start;
            let end = token.span.end;
            Some(Parsed {
                token: Token {
                    span: start..end,
                    gram: Some(name.to_string()),
                    tags: vec![],
                    meta: Default::default(),
                    children: vec![token],
                },
                diagnostics,
                incomplete,
            })
        } else {
            None
        };
        if !state.cache.contains_key(&cache_key) {
            state.cache.insert(cache_key.clone(), parsed.clone()); // TODO avoid cloning id
        }
        StackPoll::Finished(parsed)
    }

    fn poll_tagged(
        next: Option<Parsed<&'a Node<T>>>,
        tag: String,
    ) -> StackPoll<&'a Node<T>> {
        if let Some(mut next) = next {
            next.token.tags.push(tag);
            StackPoll::Finished(Some(next))
        } else {
            StackPoll::Finished(None)
        }
    }

    fn poll_meta(
        next: Option<Parsed<&'a Node<T>>>,
        meta: BTreeMap<String, String>,
    ) -> StackPoll<&'a Node<T>> {
        if let Some(mut next) = next {
            next.token.meta.extend(meta);
            StackPoll::Finished(Some(next))
        } else {
            StackPoll::Finished(None)
        }
    }

    fn poll_sequence(
        next: Option<Parsed<&'a Node<T>>>,
        elements: &'a[Node<T>],
        mut parsed: Vec<Token>,
        mut diagnostics: Vec<Diagnostic>,
    ) -> StackPoll<&'a Node<T>>{
        assert_ne!(elements.len(), 0, "Empty sequence");
        // TODO report incomplete sequence
        if let Some(Parsed { token, diagnostics: sub_diag, incomplete }) = next {
            parsed.push(token);
            diagnostics.extend(sub_diag);
            if elements.len() == parsed.len() {
                let start = parsed.first().unwrap().span.start;
                let end = parsed.last().unwrap().span.end;
                return StackPoll::Finished(Some(Parsed {
                    token: Token {
                        span: start..end,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: parsed,
                    },
                    diagnostics,
                    incomplete,
                }));
            } else {
                let n = parsed.len();
                let end = parsed.last().unwrap().span.end;
                return StackPoll::Feed(
                    Self::ParsingSequence {
                        elements,
                        parsed,
                        diagnostics,
                    },
                    &elements[n],
                    end,
                );
            }
        } else {
            if parsed.is_empty() {
                return StackPoll::Finished(None);
            } else {
                let start = parsed.first().unwrap().span.start;
                let end = parsed.last().unwrap().span.end;
                let n = parsed.len();
                let expected = &elements[n];
                // TODO more specific error
                diagnostics.push(Diagnostic::Incomplete {
                    span: end..end,
                    expected: expected.to_ebnf(),
                });
                return StackPoll::Finished(Some(Parsed {
                    token: Token {
                        span: start..end,
                        gram: None,
                        tags: vec![],
                        meta: Default::default(),
                        children: parsed,
                    },
                    diagnostics,
                    incomplete: Some(expected),
                }));
            }
        }
    }
}

impl<'a, T: TerminalNode + 'static> AbstractStackState<&'a Node<T>> for StackState<'a, T> {
    fn name(&self) -> Option<String> {
        if let Self::ParsingNonTerminal { name, .. } = self {
            Some(name.to_string())
        } else {
            None
        }
    }

    fn poll(self, next: Option<Parsed<&'a Node<T>>>, state: &mut State<'a, T>) -> StackPoll<&'a Node<T>> {
        match self {
            Self::ParsingSequence { elements, parsed, diagnostics } => {
                Self::poll_sequence(next, elements, parsed, diagnostics)
            },
            Self::ParsingChoice { start_pos, elements, current, parsed } => {
                Self::poll_choice(next, start_pos, elements, current, parsed)
            },
            Self::ParsingRepetition { element, range, parsed, start_pos, diagnostics } => {
                Self::poll_repetition(next, element, range, parsed, start_pos, diagnostics)
            },
            Self::ParsingNonTerminal { start_pos, name } => {
                Self::poll_non_terminal(next, name, start_pos, state)
            },
            Self::ParsingTagged { tag } => {
                Self::poll_tagged(next, tag)
            },
            Self::ParsingMeta { meta } => {
                Self::poll_meta(next, meta)
            },
        }
    }
}