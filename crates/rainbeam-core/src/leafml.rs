//! # Leaf Markup Language
//!
//! Simple format for defining structures which compile to HTML
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub trait CompileHTML {
    fn compile(self) -> String;
}

/// A structure containing all pages and information about the document
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plant {
    /// The "pages" in the document
    #[serde(default)]
    pub pages: HashMap<String, Leaf>,
    /// The CSS styling of the document
    #[serde(default)]
    pub styles: String,
}

impl Default for Plant {
    fn default() -> Self {
        Self {
            pages: {
                let mut out = HashMap::new();
                out.insert("home".to_string(), Leaf::default());
                out
            },
            styles: String::new(),
        }
    }
}

impl CompileHTML for Plant {
    fn compile(self) -> String {
        let mut out: String = format!(
            "<style id=\"leaf-document-styles\">{}</style>\n",
            self.styles
        );

        // compile each page
        for page in self.pages {
            out += &format!(
                "<div class=\"leaf-page hidden\" id=\"{}\">{}</div>",
                page.0,
                page.1.compile()
            );
        }

        // return
        out
    }
}

/// A single page in a [`Plant`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Leaf {
    /// The components that make up the page
    #[serde(default)]
    pub components: Vec<Cell>,
}

impl Default for Leaf {
    fn default() -> Self {
        Self {
            components: vec![Cell {
                r#type: CellType::Section,
                id: "document".to_string(),
                class_name: String::new(),
                attributes: HashMap::new(),
                styles: "margin-top: 1rem".to_string(),
                children: vec![Cell {
                    r#type: CellType::Container,
                    id: String::new(),
                    class_name: "card w-full".to_string(),
                    attributes: HashMap::new(),
                    styles: String::new(),
                    children: vec![Cell {
                        r#type: CellType::Text,
                        id: String::new(),
                        class_name: String::new(),
                        attributes: HashMap::new(),
                        styles: String::new(),
                        children: Vec::new(),
                        inner_text: "Hello, world!".to_string(),
                    }],
                    inner_text: String::new(),
                }],
                inner_text: String::new(),
            }],
        }
    }
}

impl CompileHTML for Leaf {
    fn compile(self) -> String {
        let mut out: String = String::new();

        // compile each component
        for (i, component) in self.components.iter().enumerate() {
            out += &format!(
                "<div class=\"leaf-component\" id=\"{i}\">{}</div>",
                // TODO: don't clone here
                component.to_owned().compile()
            );
        }

        // return
        out
    }
}

/// A type of [`Cell`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CellType {
    Text,
    Section,
    Container,
}

impl Default for CellType {
    fn default() -> Self {
        Self::Container
    }
}

/// A single component in a [`Leaf`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    /// The type of the cell
    #[serde(default)]
    pub r#type: CellType,
    /// The ID of the cell
    #[serde(default)]
    pub id: String,
    /// The className of the cell
    #[serde(default)]
    pub class_name: String,
    /// The attributes of the cell
    #[serde(default)]
    pub attributes: HashMap<String, String>,
    /// CSS styling for the cell
    #[serde(default)]
    pub styles: String,
    /// The children of the cell
    #[serde(default)]
    pub children: Vec<Cell>,
    /// The innerText of the cell
    #[serde(default)]
    pub inner_text: String,
}

impl CompileHTML for Cell {
    fn compile(self) -> String {
        let mut out: String = String::new();

        // tag
        let tag: &str = match self.r#type {
            CellType::Text => "span",
            CellType::Section => "main",
            CellType::Container => "div",
        };

        out += &format!("<{tag}");

        // attributes
        for attribute in self.attributes {
            out += &format!(" {}=\"{}\"", attribute.0, attribute.1);
        }

        // id
        out += &format!(" id=\"{}\"", self.id);

        // class_name
        out += &format!(" class=\"{}\"", self.class_name);

        // styles
        out += &format!(" style=\"{}\"", self.styles);

        // close
        out += ">";

        // children
        for child in self.children {
            out += &child.compile();
        }

        if !self.inner_text.is_empty() {
            out += &shared::ui::render_markdown(&self.inner_text);
        }

        // close
        out += &format!("</{tag}>");

        // return
        out
    }
}
