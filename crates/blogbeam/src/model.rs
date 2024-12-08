use serde::{Deserialize, Serialize};

/// A static site organized into blocks which compile to HTML
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Post {
    /// The ID of the site
    pub id: String,
    /// The **unique** slug of the site
    pub slug: String,
    /// The owner of the site (also the only one who can edit it)
    pub owner: String,
    /// The time in which the site was created
    pub published: u128,
    /// The time in which the site was edited
    pub edited: u128,
    /// The content of the site
    pub content: String,
    /// Additional context for the site
    pub context: PostContext,
}

/// Additional information about a [`Site`]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostContext {}

// props

#[derive(Serialize, Deserialize, Debug)]
pub struct PostCreate {
    pub slug: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostEdit {
    pub content: String,
}

/// General API errors
#[derive(Debug)]
pub enum DatabaseError {
    InvalidNameUnique,
    ContentTooShort,
    ContentTooLong,
    InvalidName,
    NotAllowed,
    ValueError,
    NotFound,
    Other,
}

impl DatabaseError {
    pub fn to_string(&self) -> String {
        use DatabaseError::*;
        match self {
            ContentTooShort => String::from("Content too short!"),
            ContentTooLong => String::from("Content too long!"),
            InvalidName => String::from("This name cannot be used!"),
            NotAllowed => String::from("You are not allowed to do this!"),
            ValueError => String::from("One of the field values given is invalid!"),
            NotFound => {
                String::from("Nothing with this path exists or you do not have access to it!")
            }
            _ => String::from("An unspecified error has occured"),
        }
    }
}
