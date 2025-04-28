use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::TerminalNode;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Encode, Decode)]
#[derive(Serialize, Deserialize)]
#[serde(into = "String")]
#[serde(from = "String")]
pub enum Text {
    /// A string terminal
    String(String),
    /// A regex terminal
    Regex(String),
}

impl TerminalNode for Text {
    type Src = str;
    fn parses(&self, src: &Self::Src, pos: usize) -> anyhow::Result<Option<usize>> {
        let r = match self {
            Text::String(s) => {
                let start = pos;
                let end = pos + s.len();
                if end <= src.len() && &src[start..end] == s {
                    Some(end)
                } else {
                    None
                }
            },
            Text::Regex(re) => 'a: {
                // TODO some caching
                let re = regex::Regex::new(re).map_err(|e| anyhow::anyhow!("Invalid regex: {e}"))?;
                if let Some(mat) = re.captures(&src[pos..]) {
                    if mat.get(0).map_or(false, |m| m.start() == 0) {
                        break 'a Some(pos + mat.get(0).unwrap().end());
                    }
                }
                None
            },
        };
        Ok(r)
    }
    fn to_ebnf(&self) -> String {
        match self {
            Text::String(s) => format!("{s:?}"),
            Text::Regex(s) => format!("/{s}/"),
        }
    }
}

impl Into<String> for Text {
    fn into(self) -> String {
        match self {
            Text::String(s) => format!("{s}"),
            Text::Regex(s) => format!("/{s}/"),
        }
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        if value.starts_with('/') && value.ends_with('/') {
            Text::Regex(value[1..value.len()-1].to_string())
        } else {
            Text::String(value)
        }
    }
}