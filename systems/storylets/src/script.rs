//! A Pack's story written as data rather than code: the same builder calls
//! the Pack's Rust uses (`spec(...)`, `said(...)`, `mark(...)`), written as
//! a JSON call tree the Pack loads at build time (`include_str!`), parses
//! once and evaluates with its own builders. What a story says and does
//! lives in the data; what each builder means stays in the Pack.
//!
//! The encoding:
//! - `{"f": [a, b]}` is the call `f(a, b)`;
//! - `{".m": [x, a]}` is the method call `x.m(a)`;
//! - `{"A{}": {"f": x}}` is the struct `A { f: x }`;
//! - `{"()": [a, b]}` is the tuple `(a, b)`;
//! - a JSON array is a list, a number an integer;
//! - `"@NAME"` is a name the Pack gives a meaning (an entity, a key,
//!   `None`), and any other string is text (`"@@..."` is text that starts
//!   with `@`);
//! - `{"//": ["a note", x]}` is `x`, with a note for whoever reads the data.

/// One node of a story's call tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A call, or a method call with its receiver first.
    Call(&'static str, Vec<Node>),
    Struct(&'static str, Vec<(&'static str, Node)>),
    List(Vec<Node>),
    Tuple(Vec<Node>),
    Text(&'static str),
    Int(i64),
    Name(&'static str),
}

fn leak(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

/// Parses a story's call tree. Its text lives for as long as the program,
/// so a Pack parses it once.
pub fn parse(json: &str) -> Result<Node, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    node(&value)
}

fn node(value: &serde_json::Value) -> Result<Node, String> {
    use serde_json::Value;
    Ok(match value {
        Value::Array(items) => Node::List(items.iter().map(node).collect::<Result<_, _>>()?),
        Value::Number(number) => Node::Int(
            number
                .as_i64()
                .ok_or_else(|| format!("{number} is not an integer"))?,
        ),
        Value::String(text) => match text.strip_prefix('@') {
            Some(rest) if rest.starts_with('@') => Node::Text(leak(rest)),
            Some(name) => Node::Name(leak(name)),
            None => Node::Text(leak(text)),
        },
        Value::Object(map) if map.len() == 1 => {
            let (key, inner) = map.iter().next().unwrap();
            if let Some(name) = key.strip_suffix("{}") {
                let Value::Object(fields) = inner else {
                    return Err(format!("{key} wants fields"));
                };
                Node::Struct(
                    leak(name),
                    fields
                        .iter()
                        .map(|(field, value)| Ok((leak(field), node(value)?)))
                        .collect::<Result<_, String>>()?,
                )
            } else {
                let Value::Array(args) = inner else {
                    return Err(format!("{key} wants a list of arguments"));
                };
                if key == "//" {
                    return match args.as_slice() {
                        [Value::String(_), inner] => node(inner),
                        _ => Err("a note wants its text and what it is about".into()),
                    };
                }
                let args = args.iter().map(node).collect::<Result<_, _>>()?;
                if key == "()" {
                    Node::Tuple(args)
                } else {
                    Node::Call(leak(key), args)
                }
            }
        }
        other => return Err(format!("not a story node: {other}")),
    })
}

impl Node {
    pub fn text(&self) -> &'static str {
        match self {
            Node::Text(text) => text,
            other => panic!("text wanted, not {other:?}"),
        }
    }

    pub fn int(&self) -> i64 {
        match self {
            Node::Int(int) => *int,
            other => panic!("an integer wanted, not {other:?}"),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Node::Name(name) => name,
            other => panic!("a name wanted, not {other:?}"),
        }
    }

    /// A list's items, or a tuple's.
    pub fn items(&self) -> &[Node] {
        match self {
            Node::List(items) | Node::Tuple(items) => items,
            other => panic!("a list wanted, not {other:?}"),
        }
    }

    /// A struct's field.
    pub fn field(&self, name: &str) -> &Node {
        match self {
            Node::Struct(_, fields) => fields
                .iter()
                .find(|(field, _)| *field == name)
                .map(|(_, value)| value)
                .unwrap_or_else(|| panic!("no field {name} in {self:?}")),
            other => panic!("a struct wanted, not {other:?}"),
        }
    }

    /// `Some(n)` or `None`, as an optional integer.
    pub fn optional_int(&self) -> Option<i64> {
        match self {
            Node::Name("None") => None,
            Node::Call("Some", args) if args.len() == 1 => Some(args[0].int()),
            other => panic!("Some(n) or None wanted, not {other:?}"),
        }
    }

    /// Every call's name in the tree, for a test that a Pack knows them all.
    pub fn calls(&self, into: &mut std::collections::BTreeSet<&'static str>) {
        match self {
            Node::Call(name, args) => {
                into.insert(name);
                for arg in args {
                    arg.calls(into);
                }
            }
            Node::Struct(name, fields) => {
                into.insert(name);
                for (_, value) in fields {
                    value.calls(into);
                }
            }
            Node::List(items) | Node::Tuple(items) => {
                for item in items {
                    item.calls(into);
                }
            }
            Node::Text(_) | Node::Int(_) | Node::Name(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_tree_parses() {
        let tree = parse(
            r#"[{"//": ["a note", {"f": ["a", 2, "@NAME", "@@x", {"()": [1, 2]}]}]},
                {".m": [{"g": []}, []]}, {"A{}": {"k": "v"}}]"#,
        )
        .unwrap();
        assert_eq!(
            tree,
            Node::List(vec![
                Node::Call(
                    "f",
                    vec![
                        Node::Text("a"),
                        Node::Int(2),
                        Node::Name("NAME"),
                        Node::Text("@x"),
                        Node::Tuple(vec![Node::Int(1), Node::Int(2)]),
                    ]
                ),
                Node::Call(".m", vec![Node::Call("g", vec![]), Node::List(vec![])]),
                Node::Struct("A", vec![("k", Node::Text("v"))]),
            ])
        );
        assert!(parse(r#"{"f": 1, "g": 2}"#).is_err());
    }
}
