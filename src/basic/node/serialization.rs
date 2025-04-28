use std::{collections::BTreeMap, fmt, ops::RangeInclusive};

use serde::{de::{self, MapAccess, Visitor}, ser::SerializeMap, Deserialize, Serialize, Serializer};

use super::Node;


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Rep<T: Clone> {
    node: Box<Node<T>>,
    #[serde(serialize_with = "super::super::serde_span_serialization::serialize")]
    #[serde(deserialize_with = "super::super::serde_span_serialization::deserialize")]
    range: RangeInclusive<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tagged<T: Clone> {
    node: Box<Node<T>>,
    tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Meta<T: Clone> {
    node: Box<Node<T>>,
    data: BTreeMap<String, String>,
}

impl<T: Serialize + Clone> Serialize for Node<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Node::Seq(nodes) => map.serialize_entry("seq", nodes)?,
            Node::Alt(nodes) => map.serialize_entry("alt", nodes)?,
            Node::Rep { node, range } if *range == (0..=1) => map.serialize_entry("opt", node)?,
            Node::Rep { node, range } => map.serialize_entry("rep", &Rep { node: node.clone(), range: range.clone() })?,
            Node::Terminal(value) => map.serialize_entry("term", value)?,
            Node::NonTerm(value) => map.serialize_entry("non_term", value)?,
            Node::Tagged { node, tag } => map.serialize_entry("tagged", &Tagged { node: node.clone(), tag: tag.clone() })?,
            Node::Meta { node, meta } => map.serialize_entry("meta", &Meta { node: node.clone(), data: meta.clone() })?,
        }
        map.end()
    }
}

impl<'de, T: Deserialize<'de> + Clone> Deserialize<'de> for Node<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NodeVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T: Deserialize<'de> + Clone> Visitor<'de> for NodeVisitor<T> {
            type Value = Node<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with a single key representing a Node variant")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expected a single key in the map"))?;
                match key.as_str() {
                    "seq" => {
                        let value = map.next_value()?;
                        Ok(Node::Seq(value))
                    }
                    "alt" => {
                        let value = map.next_value()?;
                        Ok(Node::Alt(value))
                    }
                    "opt" => {
                        let value: Node<T> = map.next_value()?;
                        Ok(Node::Rep { node: Box::new(value), range: 0..=1 })
                    }
                    "rep" => {
                        let rep: Rep<T> = map.next_value()?;
                        Ok(Node::Rep { node: rep.node, range: rep.range })
                    }
                    "term" => {
                        let value = map.next_value()?;
                        Ok(Node::Terminal(value))
                    }
                    "non_term" => {
                        let value = map.next_value()?;
                        Ok(Node::NonTerm(value))
                    }
                    "tagged" => {
                        let tagged: Tagged<T> = map.next_value()?;
                        Ok(Node::Tagged { node: tagged.node, tag: tagged.tag })
                    }
                    _ => Err(de::Error::unknown_field(&key, &[
                        "seq", "alt", "rep", "term", "re", "non_term", "tagged",
                    ])),
                }
            }
        }

        deserializer.deserialize_map(NodeVisitor::<T>(std::marker::PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use crate::basic::Text;

    use super::*;

    #[test]
    fn yaml_serialization() {
        let node = Node::Seq(vec![
            Node::Terminal(Text::String("foo".to_string())),
            Node::rep(Node::Terminal(Text::String("bar".to_string())), 1..),
        ]);

        let serialized = serde_yaml::to_string(&node).unwrap();
        assert_eq!(
            serialized,
            r#"seq:
- term: foo
- rep:
    node:
      term: bar
    range:
      min: 1
"#
        );
        println!("Serialized: {}", serialized);

        let deserialized: Node<Text> = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(node, deserialized);
    }
}