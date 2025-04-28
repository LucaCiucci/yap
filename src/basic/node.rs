use std::{collections::BTreeMap, fmt, ops::{RangeBounds, RangeInclusive}};

use bincode::{Decode, Encode};

use crate::parsers::naive::{AbstractNode, Action, Parsed};

use super::{TerminalNode, Token};

mod serialization;

mod parse_state;

pub use parse_state::*;

/// A grammar node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Encode, Decode)]
pub enum Node<T> {
    /// A sequence of nodes
    Seq(Vec<Node<T>>),
    /// A choice between nodes
    Alt(Vec<Node<T>>),
    /// A repetition of nodes
    Rep { node: Box<Node<T>>, range: RangeInclusive<usize> },
    /// A terminal node
    Terminal(T),
    /// A non-terminal node
    NonTerm(String),
    /// A string tag attached to a node
    Tagged { node: Box<Node<T>>, tag: String },
    /// Meta information
    Meta { node: Box<Node<T>>, meta: BTreeMap<String, String> },
}

impl<T> Node<T> {
    pub fn rep(node: impl Into<Node<T>>, range: impl RangeBounds<usize>) -> Self {
        let start = match range.start_bound() {
            std::ops::Bound::Included(&start) => start,
            std::ops::Bound::Excluded(&start) => start + 1, // TODO check for overflow
            std::ops::Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(&end) => end,
            std::ops::Bound::Excluded(&end) => end - 1, // TODO check for overflow
            std::ops::Bound::Unbounded => usize::MAX,
        };
        Self::Rep { node: Box::new(node.into()), range: start..=end }
    }

    pub fn tagged(node: impl Into<Node<T>>, tag: impl Into<String>) -> Self {
        Self::Tagged {
            node: Box::new(node.into()),
            tag: tag.into(),
        }
    }

    pub fn rename_reference(&mut self, old_name: &str, new_name: &str) {
        match self {
            Node::Seq(elements) => {
                for element in elements {
                    element.rename_reference(old_name, new_name);
                }
            }
            Node::Alt(branches) => {
                for branch in branches {
                    branch.rename_reference(old_name, new_name);
                }
            }
            Node::Rep { node, .. } => {
                node.rename_reference(old_name, new_name);
            }
            Node::Terminal(_) => {}
            Node::NonTerm(name) if name == old_name => {
                *name = new_name.to_string();
            }
            Node::NonTerm(_) => {}
            Node::Tagged { node, .. } => {
                node.rename_reference(old_name, new_name);
            }
            Node::Meta { node, .. } => {
                node.rename_reference(old_name, new_name);
            }
        }
    }

    pub fn to_ebnf(&self) -> String
    where
        T: TerminalNode,
    {
        match self {
            Node::Seq(nodes) => nodes.iter().map(|n| n.to_ebnf()).collect::<Vec<_>>().join(" "),
            Node::Alt(nodes) => nodes.iter().map(|n| n.to_ebnf()).collect::<Vec<_>>().join(" | "),
            Node::Rep { node, range } => {
                match (*range.start(), *range.end()) {
                    (0, 1) => format!("[{}]", node.to_ebnf()),
                    (1, usize::MAX) => format!("{}+", node.to_ebnf()),
                    (0, usize::MAX) => format!("{}*", node.to_ebnf()),
                    _ => panic!("Unsupported repetition range in EBNF: {:?}", range),
                }
            }
            Node::Terminal(value) => value.to_ebnf(),
            Node::NonTerm(name) => name.clone(),
            Node::Tagged { node, .. } => node.to_ebnf(),
            Node::Meta { node, .. } => node.to_ebnf(),
        }
    }
}


impl<'a, T: TerminalNode + 'static> AbstractNode for &'a Node<T> {
    type Src = T::Src;
    type State = State<'a, T>;
    type StackState = StackState<'a, T>;
    type Token = Token;
    fn action(
        self,
        src: &T::Src,
        pos: usize,
        state: &mut Self::State,
    ) -> anyhow::Result<Action<Self>> {
        let action = match self {
            Node::Seq(seq) => {
                Action::Push {
                    save_state: StackState::ParsingSequence {
                        elements: seq,
                        parsed: vec![],
                        diagnostics: vec![],
                    },
                    next_node: &seq[0], // TODO: check for empty sequence
                    next_pos: pos,
                }
            },
            Node::Alt(seq) => {
                Action::Push {
                    save_state: StackState::ParsingChoice {
                        start_pos: pos,
                        elements: seq,
                        current: 0,
                        parsed: vec![],
                    },
                    next_node: &seq[0], // TODO: check for empty choice
                    next_pos: pos,
                }
            },
            Node::Rep { node, range } => {
                let save_state = StackState::ParsingRepetition {
                    element: node,
                    range: range.clone(),
                    parsed: vec![],
                    start_pos: pos,
                    diagnostics: vec![],
                };
                Action::Push {
                    save_state,
                    next_node: &**node,
                    next_pos: pos,
                }
            },
            Node::Terminal(t) => {
                let parsed = if let Some(end) = t.parses(src, pos)? {
                    Some(Parsed {
                        token: Token {
                            span: pos..end,
                            gram: None,
                            tags: vec![],
                            meta: Default::default(),
                            children: vec![],
                        },
                        diagnostics: vec![],
                        incomplete: None, // TODO
                    })
                } else {
                    None
                };
                Action::Pop {
                    parsed,
                }
            },
            Node::NonTerm(name) => {
                let cache_key = (name.clone(), pos);
                if let Some(cached) = state.cache.get(&cache_key) {
                    return Ok(Action::Pop {
                        parsed: cached.clone(),
                    });
                }
                let node = state.grammar.rules.get(name).ok_or_else(|| {
                    anyhow::anyhow!("No rule for non-terminal {name:?}")
                })?;
                Action::Push {
                    save_state: StackState::ParsingNonTerminal {
                        start_pos: pos,
                        name,
                    },
                    next_node: node,
                    next_pos: pos,
                }
            },
            Node::Tagged{ node, tag } => {
                Action::Push {
                    save_state: StackState::ParsingTagged {
                        tag: tag.clone(),
                    },
                    next_node: &**node,
                    next_pos: pos,
                }
            },
            Node::Meta{ node, meta } => {
                Action::Push {
                    save_state: StackState::ParsingMeta {
                        meta: meta.clone(),
                    },
                    next_node: &**node,
                    next_pos: pos,
                }
            },
        };

        Ok(action)
    }
}

impl<T: TerminalNode> fmt::Display for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.to_ebnf().fmt(f)
    }
}