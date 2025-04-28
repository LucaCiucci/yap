use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::parsers::naive;

use super::{Node, State, TerminalNode, Text, Token};


/// A grammar
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
#[derive(Encode, Decode)]
pub struct Grammar<T: Clone> {
    pub start: Option<String>,
    pub rules: BTreeMap<String, Node<T>>,
}

impl<T: TerminalNode> Grammar<T> {
    pub fn new() -> Self {
        Self {
            start: None,
            rules: Default::default(),
        }
    }

    pub fn add_element(&mut self, name: impl Into<String>, element: impl Into<Node<T>>) -> anyhow::Result<()> {
        let name = name.into();
        let element = element.into();
        let prev = self.rules.remove(&name);
        if let Some(prev) = prev {
            if prev != element {
                return Err(anyhow::anyhow!(
                    "Element {name} already exists and is different: {prev:#?} != {element:#?}"
                ));
            } else {
                log::warn!("Element {name} already exists, and is the same");
            }
        }
        self.rules.insert(name.clone(), element);
        Ok(())
    }

    pub fn merge(mut self, other: Self) -> anyhow::Result<Self> {
        for (name, element) in other.rules.into_iter() {
            self.add_element(name, element)?;
        }
        Ok(self)
    }

    pub fn has(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }

    pub fn with_renamed_element(
        mut self,
        name: impl Into<String>,
        new_name: impl Into<String>,
    ) -> anyhow::Result<Self> {
        let name = name.into();
        let new_name = new_name.into();

        if !self.rules.contains_key(&name) {
            return Err(anyhow::anyhow!("Element '{}' not found", name));
        }

        if self.rules.contains_key(&new_name) {
            return Err(anyhow::anyhow!("Element '{}' already exists", new_name));
        }

        let element = self.rules.remove(&name).unwrap();
        self.rules.insert(new_name.clone(), element);

        for (_, element) in self.rules.iter_mut() {
            element.rename_reference(&name, &new_name);
        }

        Ok(self)
    }

    pub fn to_ebnf(&self, aligned: bool) -> String {
        let mut ebnf = String::new();
        if !aligned {
            for (name, element) in &self.rules {
                ebnf.push_str(&format!("{} = {};\n", name, element.to_ebnf()));
            }
        } else {
            let len = |s: &String| s.chars().count();
            let max_len = self.rules.keys().map(len).max().unwrap_or(0);
            for (name, element) in &self.rules {
                let padding = " ".repeat(max_len - len(name));
                ebnf.push_str(&format!("{}{} = {};\n", name, padding, element.to_ebnf()));
            }
        }
        ebnf
    }

    pub fn parse_non_term(
        &self,
        non_term: &str,
        source: &T::Src,
    ) -> anyhow::Result<Option<(Token, Vec<naive::Diagnostic>)>> {
        self.parse_node(
            self.rules.get(non_term).ok_or_else(|| {
                anyhow::anyhow!("No rule for start node {non_term:?}")
            })?,
            source,
        )
    }

    pub fn parse_node(
        &self,
        node: &Node<T>,
        source: &T::Src,
    ) -> anyhow::Result<Option<(Token, Vec<naive::Diagnostic>)>> {
        naive::parse_recursive(
            source,
            node,
            State::new(self),
        )
    }
}

impl Grammar<Text> {
    pub fn load_ebnf(source: &str) -> anyhow::Result<Self> {
        let result = ebnf::get_grammar(source)
            .map_err(|e| anyhow::anyhow!("Failed to parse EBNF: {e}"))?;

        use ebnf::{Node as EbnfNode, RegexExtKind, SymbolKind};
        fn node_to_gram(node: EbnfNode) -> Node<Text> {
            match node {
                EbnfNode::String(s) => Node::Terminal(Text::String(s)),
                EbnfNode::RegexString(re) => Node::Terminal(Text::Regex(re)),
                EbnfNode::Terminal(s) => Node::NonTerm(s),
                EbnfNode::Multiple(nodes) => {
                    let mut flattened = Vec::new();
                    for n in nodes {
                        match node_to_gram(n) {
                            Node::Alt(mut inner) => flattened.append(&mut inner),
                            other => flattened.push(other),
                        }
                    }
                    Node::Alt(flattened)
                }
                EbnfNode::RegexExt(node, kind) => match kind {
                    RegexExtKind::Repeat0 => Node::rep(node_to_gram(*node), 0..),
                    RegexExtKind::Repeat1 => Node::rep(node_to_gram(*node), 1..),
                    RegexExtKind::Optional => Node::rep(node_to_gram(*node), 0..=1),
                },
                EbnfNode::Symbol(a, kind, b) => match kind {
                    SymbolKind::Concatenation => {
                        let mut elements = Vec::new();
                        match node_to_gram(*a) {
                            Node::Seq(mut inner) => elements.append(&mut inner),
                            other => elements.push(other),
                        }
                        match node_to_gram(*b) {
                            Node::Seq(mut inner) => elements.append(&mut inner),
                            other => elements.push(other),
                        }
                        Node::Seq(elements)
                    }
                    SymbolKind::Alternation => {
                        let mut branches = Vec::new();
                        match node_to_gram(*a) {
                            Node::Alt(mut inner) => branches.append(&mut inner),
                            other => branches.push(other),
                        }
                        match node_to_gram(*b) {
                            Node::Alt(mut inner) => branches.append(&mut inner),
                            other => branches.push(other),
                        }
                        Node::Alt(branches)
                    }
                },
                EbnfNode::Group(node) => node_to_gram(*node),
                EbnfNode::Optional(node) => Node::rep(node_to_gram(*node), 0..=1),
                EbnfNode::Repeat(node) => Node::rep(node_to_gram(*node), 0..),
                EbnfNode::Unknown => panic!("Unknown EBNF node encountered"),
            }
        }

        let mut grammar = Grammar::new();

        for expr in result.expressions {
            grammar.rules.insert(expr.lhs, node_to_gram(expr.rhs));
        }

        Ok(grammar)
    }
}