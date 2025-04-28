/*!
Naive iterative parser for EBNF-like grammars.
*/

use std::{fmt::{self, Debug}, ops::Range};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Diagnostic {
    Incomplete {
        span: Range<usize>,
        expected: String,
    },
}

impl Diagnostic { // TODO remove TerminalNode bound
    pub fn main_span(&self) -> Range<usize> {
        match self {
            Diagnostic::Incomplete { span, .. } => span.clone(),
        }
    }
    pub fn message(&self) -> String {
        match self {
            Diagnostic::Incomplete { span, expected } => format!("Incomplete parse at {}: expected {expected}", span.start),
        }
    }
}

// TODO into a parser struct
pub fn parse_recursive<N: AbstractNode + Debug>(
    source: &N::Src,
    start: N,
    mut state: N::State,
) -> anyhow::Result<Option<(N::Token, Vec<Diagnostic>)>> {
    let mut stack: Vec<N::StackState> = vec![];

    // initialization
    let mut curr_step = Step::ParsingNode {
        node: start,
        pos: 0,
    };

    // parsing loop
    let parsed = 'a: loop {
        check_stack(&stack)?;

        curr_step = match curr_step {
            Step::ParsingNode { node, pos } => {
                let action = node.action(source, pos, &mut state)?;
                match action {
                    Action::Push { save_state, next_node, next_pos } => {
                        stack.push(save_state);
                        Step::ParsingNode {
                            node: next_node,
                            pos: next_pos,
                        }
                    },
                    Action::Pop { parsed } => {
                        Step::Polling { parsed }
                    },
                }
            },
            Step::Polling { parsed } => if let Some(stack_state) = stack.pop() {
                let poll = stack_state.poll(parsed, &mut state);
                match poll {
                    StackPoll::Finished(parsed) => {
                        Step::Polling { parsed } // ! pos???
                    },
                    StackPoll::Feed(state, node, pos) => {
                        stack.push(state);
                        Step::ParsingNode {
                            node,
                            pos, // ! pos???
                        }
                    },
                }
            } else {
                break 'a parsed;
            },
        };
    };

    Ok(parsed.map(|t| {
        (t.token, t.diagnostics)
    }))
}

/// Check the stack for recursion limit
fn check_stack<'a, N: AbstractNode, S: AbstractStackState<N>>(
    stack: &[S],
) -> anyhow::Result<()> {
    if stack.len() > 1000 {
        let mut non_term_stack = vec![];
        for elem in stack.iter() {
            if let Some(name) = elem.name() {
                non_term_stack.push(name);
            }
        }
        return Err(anyhow::anyhow!("Recursion limit exceeded: stack is {:#?}", non_term_stack));
    }

    Ok(())
}

#[derive(Debug)]
pub enum Action<Node: AbstractNode> {
    Push {
        save_state: Node::StackState,
        next_node: Node,
        next_pos: usize,
    },
    Pop {
        parsed: Option<Parsed<Node>>,
    },
}

pub enum StackPoll<N: AbstractNode> {
    /// Parse completed
    Finished(Option<Parsed<N>>),
    /// Continue parsing at position
    Feed(N::StackState, N, usize),
}

pub trait AbstractStackState<N: AbstractNode>: Debug + Sized { // TODO remove debug
    fn name(&self) -> Option<String>;
    fn poll(self, next: Option<Parsed<N>>, state: &mut N::State) -> StackPoll<N>;
}

pub trait AbstractNode: Sized + Clone + fmt::Display { // TODO remove clone
    type Src: ?Sized;
    type State;
    type StackState: AbstractStackState<Self>;
    type Token: Sized + Clone + Debug; // TODO remove debug
    fn action(
        self,
        src: &Self::Src,
        pos: usize,
        state: &mut Self::State,
    ) -> anyhow::Result<Action<Self>>;
}

#[derive(Debug, Clone)]
pub struct Parsed<N: AbstractNode> {
    pub token: N::Token,
    pub diagnostics: Vec<Diagnostic>,
    pub incomplete: Option<N>,
}

#[derive(Debug)]
enum Step<N: AbstractNode> {
    ParsingNode {
        node: N,
        pos: usize,
    },
    Polling {
        parsed: Option<Parsed<N>>,
    }
}

#[cfg(test)]
mod tests {
    use crate::{basic::{Grammar, State}, parsers::tests::cases};

    use super::*;

    #[test]
    fn alt_term() {
        for (grammar, input, expected) in cases() {
            eprintln!("Testing input: {input:?} with {grammar:#?}");

            let result = parse_recursive(
                input,
                &grammar,
                State::new(&Grammar::new()),
            ).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_parse_complex_ebnf() {
        let source = r#"
            expression = term , (("+" | "-") , term)*;
            term = factor , (("*" | "/") , factor)*;
            factor = ("(" , expression , ")") | number;
            number = digit+;
            digit = "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";
        "#;

        let grammar = Grammar::load_ebnf(source).expect("Failed to load EBNF");
        println!("Loaded grammar: {:#?}", grammar);

        let input = "(1+2)*33";
        let start = "expression";

        let result = grammar.parse_non_term(start, input).unwrap();
        assert!(result.is_some(), "Parsing failed for input: {}", input);

        let (token, diagnostics) = result.unwrap();
        assert!(diagnostics.is_empty(), "Unexpected diagnostics: {:?}", diagnostics);

        // Additional assertions can be added here to validate the parsed token structure
        eprintln!("Parsed token: {:?}", token);
    }
}