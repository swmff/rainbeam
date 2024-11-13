use std::collections::HashMap;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use hcaptcha::Hcaptcha;
use serde::{Deserialize, Serialize};

use authbeam::model::{IpBlock, Profile, UserFollow};
use databeam::DefaultReturn;
pub use authbeam::model::RelationshipStatus;

/// Trait for simple asset contexts
pub trait Context {}

/// Trait for generic structs which contain a "content" and "context"
pub trait CtxAsset {
    fn ref_context(&self) -> &impl Context;
    fn ref_content(&self) -> &String;

    fn ref_asset(&self) -> (AssetType, &String);
}

/// A question structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Question {
    /// The author of the question; "anonymous" marks the question as an anonymous question
    pub author: Profile,
    /// The recipient of the question; cannot be anonymous
    pub recipient: Profile,
    /// The content of the question
    pub content: String,
    /// The ID of the question
    pub id: String,
    /// The IP address of the user asking the question
    #[serde(default)]
    pub ip: String,
    /// The time this question was asked
    pub timestamp: u128,
    /// Additional information about the question
    #[serde(default)]
    pub context: QuestionContext,
}

impl CtxAsset for Question {
    fn ref_context(&self) -> &impl Context {
        &self.context
    }

    fn ref_content(&self) -> &String {
        &self.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Question, &self.content)
    }
}

impl Question {
    pub fn lost(author: String, recipient: String, content: String, timestamp: u128) -> Self {
        Self {
            author: anonymous_profile(author),
            recipient: anonymous_profile(recipient),
            content,
            id: String::new(),
            ip: String::new(),
            timestamp,
            context: QuestionContext::default(),
        }
    }

    pub fn post() -> Self {
        Self {
            author: anonymous_profile("anonymous".to_string()),
            recipient: anonymous_profile("anonymous".to_string()),
            content: "<post>".to_string(),
            id: "0".to_string(),
            ip: String::new(),
            timestamp: 0,
            context: QuestionContext::default(),
        }
    }

    pub fn unknown() -> Self {
        Self::lost(
            "anonymous".to_string(),
            String::new(),
            "<lost question>".to_string(),
            0,
        )
    }
}

/// Basic information which changes the way the response is deserialized
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuestionContext {
    /// The ID of the response in which this question is replying to
    ///
    /// Will fill into the "reply" field of the response that is posted to this question
    #[serde(default)]
    pub reply_intent: String,
    /// The media property of the question
    ///
    /// Media is prefixed to decide what its type is:
    /// * (no prefix): URL
    /// * `--CARP`: carp canvas drawing
    #[serde(default)]
    pub media: String,
}

impl Context for QuestionContext {}

impl Default for QuestionContext {
    fn default() -> Self {
        Self {
            reply_intent: String::new(),
            media: String::new(),
        }
    }
}

/// A question structure with ID references to profiles instead of the profiles
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RefQuestion {
    /// The author of the question; "anonymous" marks the question as an anonymous question
    pub author: String,
    /// The recipient of the question; cannot be anonymous
    pub recipient: String,
    /// The content of the question
    pub content: String,
    /// The ID of the question
    pub id: String,
    /// The IP address of the user asking the questionn
    pub ip: String,
    /// The time this question was asked
    pub timestamp: u128,
    /// Additional information about the question
    #[serde(default)]
    pub context: QuestionContext,
}

impl From<Question> for RefQuestion {
    fn from(value: Question) -> Self {
        Self {
            author: value.author.id,
            recipient: value.recipient.id,
            content: value.content,
            id: value.id,
            ip: value.ip,
            timestamp: value.timestamp,
            context: value.context,
        }
    }
}

/// A response structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuestionResponse {
    /// The author of the response; cannot be anonymous
    pub author: Profile,
    /// The ID question this response is replying to
    pub question: String,
    /// The content of the response
    pub content: String,
    /// The ID of the response
    pub id: String,
    /// The time this response was created
    pub timestamp: u128,
    /// The response tags
    pub tags: Vec<String>,
    /// Response context
    pub context: ResponseContext,
    /// The ID of the response this response is replying to
    pub reply: String,
    /// The time this response was last edited
    pub edited: u128,
}

impl CtxAsset for QuestionResponse {
    fn ref_context(&self) -> &impl Context {
        &self.context
    }

    fn ref_content(&self) -> &String {
        &self.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Response, &self.content)
    }
}

pub type FullResponse = (Question, QuestionResponse, usize, usize);

impl CtxAsset for FullResponse {
    fn ref_context(&self) -> &impl Context {
        &self.1.context
    }

    fn ref_content(&self) -> &String {
        &self.1.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Response, &self.1.content)
    }
}

/// Basic information which changes the way the response is deserialized
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseContext {
    /// If the response is a post and the question shouldn't be rendered at all
    #[serde(default)]
    pub is_post: bool,
    /// If the response is unlisted (not shown on PUBLIC timelines/searches)
    #[serde(default)]
    pub unlisted: bool,
    /// The warning shown on the response. Users must accept this warning to view the response
    ///
    /// Empty means no warning.
    #[serde(default)]
    pub warning: String,
    /// The ID of the circle this response belongs to
    #[serde(default)]
    pub circle: String,
}

impl Context for ResponseContext {}

impl Default for ResponseContext {
    fn default() -> Self {
        Self {
            is_post: false,
            unlisted: false,
            warning: String::new(),
            circle: String::new(),
        }
    }
}

/// A comment structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseComment {
    /// The author of the comment; cannot be anonymous
    pub author: Profile,
    /// ID of the response this comment is replying to
    pub response: String,
    /// The content of the comment
    pub content: String,
    /// The ID of the comment
    pub id: String,
    /// The time this comment was created
    pub timestamp: u128,
    /// The ID of the comment this comment is replying to
    pub reply: Option<Box<ResponseComment>>,
    /// The time this comment was last edited
    pub edited: u128,
    /// The IP address of the user creating the comment
    #[serde(default)]
    pub ip: String,
    /// Extra information about the comment
    #[serde(default)]
    pub context: CommentContext,
}

impl CtxAsset for ResponseComment {
    fn ref_context(&self) -> &impl Context {
        &self.context
    }

    fn ref_content(&self) -> &String {
        &self.content
    }

    fn ref_asset(&self) -> (AssetType, &String) {
        (AssetType::Comment, &self.content)
    }
}

/// Basic information which changes the way the response is deserialized
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommentContext {}

impl Context for CommentContext {}
impl Default for CommentContext {
    fn default() -> Self {
        Self {}
    }
}

/// A reaction structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reaction {
    /// The reactor of the reaction; cannot be anonymous
    pub user: Profile,
    /// ID of the asset this reaction is on (response, comment, etc.)
    pub asset: String,
    /// The time this reaction was created
    pub timestamp: u128,
}

/// The type of any asset (anything created by a user)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AssetType {
    /// A [`Question`]
    Question,
    /// A [`QuestionResponse`]
    Response,
    /// A [`ResponseComment`]
    Comment,
}

/// The status of a user's membership in a [`Circle`]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum MembershipStatus {
    /// A user who has received an invite to a circle, but has not yet accepted
    Pending,
    /// An active member of a circle
    Active,
    /// Not pending or an active member
    Inactive,
}

/// The stored version of a user's membership in a [`Circle`]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CircleMembership {
    /// The ID of the user
    pub user: String,
    /// The ID of the circle
    pub circle: String,
    /// The status of the user's membership in the circle
    pub membership: MembershipStatus,
    /// The time the membership was last updated
    pub timestamp: u128,
}

/// A circle structure
///
/// Circles allow you to post global questions to them (recipient `@circle`),
/// as well as define a custom avatar URL, banner URL, and define a custom theme
///
/// Users can also ask a question and send it to the circle's inbox.
/// This question can then be replied to by anybody in the circle.
///
/// Users can be invited to a circle by the circle's owner. Users are added to the `xcircle_memberships`
/// table with a [`MembershipStatus`] of `Pending`. Users can accept through a notification that is sent
/// to their account, which will then change their [`MembershipStatus`] to `Active`.
///
/// Active members can post to the circle through the compose form. Memberships can always be managed
/// by the owner of the circle, who can remove anybody they want from the circle.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Circle {
    /// The name of the circle
    pub name: String,
    /// The ID of the circle
    pub id: String,
    /// The owner of the circle
    pub owner: Profile,
    /// The metadata of the circle
    pub metadata: CircleMetadata,
    /// The time the circle was created
    pub timestamp: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CircleMetadata {
    pub kv: HashMap<String, String>,
}

impl CircleMetadata {
    /// Check `kv` lengths
    ///
    /// # Returns
    /// * `true`: ok
    /// * `false`: invalid
    pub fn check(&self) -> bool {
        for field in &self.kv {
            if field.0 == "sparkler:custom_css" {
                // custom_css gets an extra long value
                if field.1.len() > 64 * 128 {
                    return false;
                }

                continue;
            }

            if field.1.len() > 64 * 64 {
                return false;
            }
        }

        true
    }
}

/// An export of a user's entire history
#[derive(Serialize, Deserialize)]
pub struct DataExport {
    /// The user's profile
    #[serde(default)]
    pub profile: Profile,
    /// All of the user's [`Question`]s
    #[serde(default)]
    pub questions: Option<Vec<(Question, usize, usize)>>,
    /// All of the user's [`QuestionResponse`]s
    #[serde(default)]
    pub responses: Option<Vec<FullResponse>>,
    /// All of the user's [`ResponseComment`]s
    #[serde(default)]
    pub comments: Option<Vec<(ResponseComment, usize, usize)>>,
    /// All of the user's [`Chat`]s
    #[serde(default)]
    pub chats: Option<Vec<(Chat, Vec<Profile>)>>,
    /// All of the user's [`Message`]s
    #[serde(default)]
    pub messages: Option<Vec<(Message, Profile)>>,
    /// Get all of the user's ipblocks
    #[serde(default)]
    pub ipblocks: Option<Vec<IpBlock>>,
    /// Get all of the user's relationships
    #[serde(default)]
    pub relationships: Option<Vec<(Profile, RelationshipStatus)>>,
    /// Get all of the user's following
    #[serde(default)]
    pub following: Option<Vec<(UserFollow, Profile, Profile)>>,
    /// Get all of the user's followers
    #[serde(default)]
    pub followers: Option<Vec<(UserFollow, Profile, Profile)>>,
}

#[derive(Serialize, Deserialize)]
pub struct DataExportOptions {
    /// Include all
    #[serde(default)]
    pub all: bool,
    /// Include `questions`
    #[serde(default)]
    pub questions: bool,
    /// Include `responses`
    #[serde(default)]
    pub responses: bool,
    /// Include `comments`
    #[serde(default)]
    pub comments: bool,
    /// Include `chats`
    #[serde(default)]
    pub chats: bool,
    /// Include `messages`
    #[serde(default)]
    pub messages: bool,
    /// Include `ipblocks`
    #[serde(default)]
    pub ipblocks: bool,
    /// Include `relationships`
    #[serde(default)]
    pub relationships: bool,
    /// Include `followers`
    #[serde(default)]
    pub followers: bool,
    /// Include `following`
    #[serde(default)]
    pub following: bool,
}

/// Direct message stream
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chat {
    /// The ID of the chat
    pub id: String,
    /// The users in the chat
    pub users: Vec<String>,
    /// The context of the chat
    pub context: ChatContext,
    /// The time the chat was created
    pub timestamp: u128,
    /// The name of the chat
    #[serde(default)]
    pub name: String,
}

/// Additional information about a [`Chat`]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatContext {}

/// Direct message
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    /// The ID of the message
    pub id: String,
    /// The ID of the chat the message is in
    pub chat: String,
    /// The user who sent the message
    pub author: String,
    /// The content of the message
    pub content: String,
    /// The context of the message
    pub context: MessageContext,
    /// The time the message was sent
    pub timestamp: u128,
    /// The time the message was edited
    pub edited: u128,
}

/// Additional information about a [`Message`]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageContext {}

/// A long blog-like post that can be edited an unlimited number of times
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Page {
    /// The ID of the page
    pub id: String,
    /// The **unique** slug of the page
    pub slug: String,
    /// The owner of the page (also the only one who can edit it)
    pub owner: String,
    /// The time in which the page was created
    pub published: u128,
    /// The time in which the page was edited
    pub edited: u128,
    /// The content of the page
    pub content: String,
    /// Additional context for the page
    pub context: PageContext,
}

/// Additional information about a [`Page`]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PageContext {}

// ...

/// Anonymous user profile
pub fn anonymous_profile(tag: String) -> Profile {
    Profile::anonymous(tag)
}

// props

#[derive(Serialize, Deserialize, Debug)]
pub struct QuestionCreate {
    pub recipient: String,
    pub content: String,
    pub anonymous: bool,
    #[serde(default)]
    pub reply_intent: String,
    #[serde(default)]
    pub media: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseCreate {
    pub question: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub warning: String,
    #[serde(default)]
    pub reply: String,
    #[serde(default)]
    pub unlisted: bool,
    #[serde(default)]
    pub circle: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseEdit {
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseEditTags {
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommentCreate {
    pub response: String,
    pub content: String,
    #[serde(default)]
    pub reply: String,
    #[serde(default)]
    pub anonymous: bool,
}

#[derive(Serialize, Deserialize, Debug, Hcaptcha)]
pub struct CircleCreate {
    pub name: String,
    #[captcha]
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EditCircleMetadata {
    pub metadata: CircleMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReactionCreate {
    pub r#type: AssetType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageCreate {
    pub chat: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatNameEdit {
    #[serde(default)]
    pub chat: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatAdd {
    #[serde(default)]
    pub chat: String,
    pub friend: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PageCreate {
    pub slug: String,
    pub content: String,
}

/// General API errors
#[derive(Debug)]
pub enum DatabaseError {
    AnonymousNotAllowed,
    InvalidNameUnique,
    ContentTooShort,
    ContentTooLong,
    ProfileLocked,
    InvalidName,
    NotAllowed,
    ValueError,
    NotFound,
    Filtered,
    Blocked,
    Banned,
    Other,
}

impl DatabaseError {
    pub fn to_string(&self) -> String {
        use DatabaseError::*;
        match self {
            AnonymousNotAllowed => {
                String::from("This profile is not currently accepting anonymous questions.")
            }
            InvalidNameUnique => String::from("This name cannot be used as it is already in use."),
            ContentTooShort => String::from("Content too short!"),
            ContentTooLong => String::from("Content too long!"),
            ProfileLocked => String::from("This profile is not currently accepting questions."),
            InvalidName => String::from("This name cannot be used!"),
            NotAllowed => String::from("You are not allowed to do this!"),
            ValueError => String::from("One of the field values given is invalid!"),
            NotFound => {
                String::from("Nothing with this path exists or you do not have access to it!")
            }
            Filtered => String::from("This content has been blocked by a content filter."),
            Blocked => String::from("You're blocked."),
            Banned => String::from("You're banned for suspected systems abuse or violating TOS."),
            _ => String::from("An unspecified error has occured"),
        }
    }
}

impl IntoResponse for DatabaseError {
    fn into_response(self) -> Response {
        use DatabaseError::*;
        match self {
            NotAllowed => (
                StatusCode::UNAUTHORIZED,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 401,
                }),
            )
                .into_response(),
            NotFound => (
                StatusCode::NOT_FOUND,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 404,
                }),
            )
                .into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DefaultReturn::<u16> {
                    success: false,
                    message: self.to_string(),
                    payload: 500,
                }),
            )
                .into_response(),
        }
    }
}

impl<T: Default> Into<databeam::DefaultReturn<T>> for DatabaseError {
    fn into(self) -> databeam::DefaultReturn<T> {
        DefaultReturn {
            success: false,
            message: self.to_string(),
            payload: T::default(),
        }
    }
}

impl From<authbeam::model::DatabaseError> for DatabaseError {
    fn from(_: authbeam::model::DatabaseError) -> Self {
        Self::Other
    }
}
