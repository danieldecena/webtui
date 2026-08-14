use serde::Deserialize;

pub const SNIPPET: &str = include_str!("extract.js");

#[derive(Debug, Clone, Deserialize)]
pub struct Document {
    pub url: String,
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Block {
    pub kind: Kind,
    pub text: String,
    pub id: Option<u32>,
    pub href: Option<String>,
    pub level: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Heading,
    Para,
    Link,
    Input,
    Button,
    Listitem,
    Code,
    Row,
}

impl Document {
    /// Ids are assigned by the snippet in document order, so this is the focus ring.
    pub fn addressable(&self) -> Vec<u32> {
        self.blocks.iter().filter_map(|b| b.id).collect()
    }
}
