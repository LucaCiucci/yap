use std::{collections::{BTreeMap, VecDeque}, ops::Range};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
#[derive(Encode, Decode)]
pub struct Token {
    pub span: Range<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gram: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Token>,
}

impl Token {
    pub fn walk_grams(
        &self,
        f: &mut dyn FnMut(&str, &Range<usize>)
    ) {
        if let Some(gram) = &self.gram {
            f(gram, &self.span);
        }
        for child in &self.children {
            child.walk_grams(f);
        }
    }

    /// Recursively iterate over all tokens with the given label in `tags`
    pub fn iter_label(
        &self,
        label: &str,
    ) -> impl Iterator<Item = &Token> {
        let mut stack = VecDeque::new();
        stack.push_back(self);
        std::iter::from_fn(move || {
            while let Some(token) = stack.pop_front() {
                if token.tags.iter().any(|tag| tag == label) {
                    stack.extend(&token.children);
                    return Some(token);
                }
                stack.extend(&token.children);
            }
            None
        })
    }

    /// Recursively iterate over all tokens with the given label in `tags`
    pub fn iter_grams(
        &self,
        gram: &str,
    ) -> impl Iterator<Item = &Token> {
        let mut stack = VecDeque::new();
        stack.push_back(self);
        std::iter::from_fn(move || {
            while let Some(token) = stack.pop_front() {
                if token.gram.as_deref() == Some(gram) {
                    stack.extend(&token.children);
                    return Some(token);
                }
                stack.extend(&token.children);
            }
            None
        })
    }

    /// Iterate over the tokens at the given position, descending
    ///
    /// The deepest token can be accessed with `token.iter_at_pos(p).last()`
    pub fn iter_at_pos(
        &self,
        pos: usize,
    ) -> impl Iterator<Item = &Token> {
        let mut current = self.span
            .contains(&pos)
            .then_some(self);

        std::iter::from_fn(move || {
            let Some(token) = current.take() else {
                return None;
            };

            for child in &token.children {
                if child.span.contains(&pos) {
                    current = Some(child);
                    break;
                }
            }

            Some(token)
        })
    }
}