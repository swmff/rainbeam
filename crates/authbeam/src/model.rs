use hcaptcha_no_wasm::Hcaptcha;
use std::collections::{BTreeMap, HashMap};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use serde::{Serialize, Deserialize};
use databeam::prelude::DefaultReturn;

use crate::layout::LayoutComponent;

/// Basic user structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    /// User ID
    pub id: String,
    /// User name
    pub username: String,
    /// Hashed user password
    pub password: String,
    /// User password salt
    pub salt: String,
    /// User login tokens
    pub tokens: Vec<String>,
    /// User IPs (these line up with the tokens in `tokens`)
    pub ips: Vec<String>,
    /// Extra information about tokens (these line up with the tokens in `tokens`)
    pub token_context: Vec<TokenContext>,
    /// Extra user information
    pub metadata: ProfileMetadata,
    /// User badges
    ///
    /// `Vec<(Text, Background, Text Color)>`
    pub badges: Vec<(String, String, String)>,
    /// User group
    pub group: i32,
    /// User join timestamp
    pub joined: u128,
    /// User tier for paid benefits
    pub tier: i32,
    /// The labels applied to the user (comma separated when as string with 1 comma at the end which creates an empty label)
    pub labels: Vec<String>,
    /// User coin balance
    pub coins: i32,
    /// User links
    pub links: BTreeMap<String, String>,
    /// User layout
    pub layout: LayoutComponent,
}

impl Profile {
    /// Global user profile
    pub fn global() -> Self {
        Self {
            username: "@".to_string(),
            id: "@".to_string(),
            password: String::new(),
            salt: String::new(),
            tokens: Vec::new(),
            ips: Vec::new(),
            token_context: Vec::new(),
            group: 0,
            joined: 0,
            metadata: ProfileMetadata::default(),
            badges: Vec::new(),
            tier: 0,
            labels: Vec::new(),
            coins: 0,
            links: BTreeMap::new(),
            layout: LayoutComponent::default(),
        }
    }

    /// System profile
    pub fn system() -> Self {
        Self {
            username: "system".to_string(),
            id: "0".to_string(),
            password: String::new(),
            salt: String::new(),
            tokens: Vec::new(),
            ips: Vec::new(),
            token_context: Vec::new(),
            group: 0,
            joined: 0,
            metadata: ProfileMetadata::default(),
            badges: Vec::new(),
            tier: 0,
            labels: Vec::new(),
            coins: 0,
            links: BTreeMap::new(),
            layout: LayoutComponent::default(),
        }
    }

    /// Anonymous user profile
    pub fn anonymous(tag: String) -> Self {
        Self {
            username: "anonymous".to_string(),
            id: tag,
            password: String::new(),
            salt: String::new(),
            tokens: Vec::new(),
            ips: Vec::new(),
            token_context: Vec::new(),
            group: 0,
            joined: 0,
            metadata: ProfileMetadata::default(),
            badges: Vec::new(),
            tier: 0,
            labels: Vec::new(),
            coins: 0,
            links: BTreeMap::new(),
            layout: LayoutComponent::default(),
        }
    }

    /// Get the tag of an anonymous ID
    ///
    /// # Returns
    /// `(is anonymous, tag, username, input)`
    pub fn anonymous_tag(input: &str) -> (bool, String, String, String) {
        if (input != "anonymous") && !input.starts_with("anonymous#") {
            // not anonymous
            return (false, String::new(), String::new(), input.to_string());
        }

        // anonymous questions from BEFORE the anonymous tag update will just have the "anonymous" tag
        let split: Vec<&str> = input.split("#").collect();
        (
            true,
            split.get(1).unwrap_or(&"unknown").to_string(),
            split.get(0).unwrap().to_string(),
            input.to_string(),
        )
    }

    /// Clean profile information
    pub fn clean(&mut self) -> () {
        self.ips = Vec::new();
        self.tokens = Vec::new();
        self.token_context = Vec::new();
        self.salt = String::new();
        self.password = String::new();
        self.metadata = ProfileMetadata::default();
    }

    /// Get context from a token
    pub fn token_context_from_token(&self, token: &str) -> TokenContext {
        let token = databeam::utility::hash(token.to_string());

        if let Some(pos) = self.tokens.iter().position(|t| *t == token) {
            if let Some(ctx) = self.token_context.get(pos) {
                return ctx.to_owned();
            }

            return TokenContext::default();
        }

        return TokenContext::default();
    }
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            username: String::new(),
            password: String::new(),
            salt: String::new(),
            tokens: Vec::new(),
            ips: Vec::new(),
            token_context: Vec::new(),
            metadata: ProfileMetadata::default(),
            badges: Vec::new(),
            group: 0,
            joined: databeam::utility::unix_epoch_timestamp(),
            tier: 0,
            labels: Vec::new(),
            coins: 0,
            links: BTreeMap::new(),
            layout: LayoutComponent::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenContext {
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub permissions: Option<Vec<TokenPermission>>,
    #[serde(default)]
    pub timestamp: u128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum TokenPermission {
    /// Manage UGC (user-generated-content) uploaded by the user
    ManageAssets,
    /// Manage user metadata
    ManageProfile,
    /// Manage all user fields
    ManageAccount,
    /// Execute moderator actions
    Moderator,
    /// Generate tokens on behalf of the account
    ///
    /// Generated tokens cannot have any permissions the token used to generate it doesn't have
    GenerateTokens,
    /// Send mail to other users on behalf of the user
    SendMail,
}

impl TokenContext {
    /// Get the value of the token's `app` field
    ///
    /// Returns an empty string if the field value is `None`
    pub fn app_name(&self) -> String {
        if let Some(ref name) = self.app {
            return name.to_string();
        }

        String::new()
    }

    /// Check if the token has the given [`TokenPermission`]
    ///
    /// ### Returns `true` if the field value is `None`
    pub fn can_do(&self, permission: TokenPermission) -> bool {
        if let Some(ref permissions) = self.permissions {
            return permissions.contains(&permission);
        }

        return true;
    }
}

impl Default for TokenContext {
    fn default() -> Self {
        Self {
            app: None,
            permissions: None,
            timestamp: databeam::utility::unix_epoch_timestamp(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileMetadata {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub policy_consent: bool,
    /// Extra key-value pairs
    #[serde(default)]
    pub kv: HashMap<String, String>,
}

impl ProfileMetadata {
    /// Check if a value exists in `kv` (and isn't empty)
    pub fn exists(&self, key: &str) -> bool {
        if let Some(ref value) = self.kv.get(key) {
            if value.is_empty() {
                return false;
            }

            return true;
        }

        false
    }

    /// Check if a value in `kv` is "true"
    pub fn is_true(&self, key: &str) -> bool {
        if !self.exists(key) {
            return false;
        }

        self.kv.get(key).unwrap() == "true"
    }

    /// Get a value from `kv`, returns an empty string if it doesn't exist
    pub fn soft_get(&self, key: &str) -> String {
        if !self.exists(key) {
            return String::new();
        }

        self.kv.get(key).unwrap().to_owned()
    }

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

impl ProfileMetadata {
    pub fn from_email(email: String) -> Self {
        Self {
            email,
            policy_consent: true,
            kv: HashMap::new(),
        }
    }
}

impl Default for ProfileMetadata {
    fn default() -> Self {
        Self {
            email: String::new(),
            policy_consent: true, // we can mark this as true since it is required for sign up
            kv: HashMap::new(),
        }
    }
}

/// Basic follow structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserFollow {
    /// The ID of the user following
    pub user: String,
    /// The ID of the user they are following
    pub following: String,
}

/// Basic notification structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Notification {
    /// The title of the notification
    pub title: String,
    /// The content of the notification
    pub content: String,
    /// The address of the notification (where it goes)
    pub address: String,
    /// The timestamp of when the notification was created
    pub timestamp: u128,
    /// The ID of the notification
    pub id: String,
    /// The recipient of the notification
    pub recipient: String,
}

/// Basic warning structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Warning {
    /// The ID of the warning
    pub id: String,
    /// The content of the warning
    pub content: String,
    /// The timestamp of when the warning was created
    pub timestamp: u128,
    /// The recipient of the warning
    pub recipient: String,
    /// The moderator who warned the recipient
    pub moderator: Box<Profile>,
}

/// Basic IP ban
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IpBan {
    /// The ID of the ban
    pub id: String,
    /// The IP that was banned
    pub ip: String,
    /// The reason for the ban
    pub reason: String,
    /// The user that banned this IP
    pub moderator: Box<Profile>,
    /// The timestamp of when the ban was created
    pub timestamp: u128,
}

/// The state of a user's relationship with another user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipStatus {
    /// No relationship
    Unknown,
    /// User two is blocked from interacting with user one
    Blocked,
    /// User two is pending a friend request from user one
    Pending,
    /// User two is friends with user one
    Friends,
}

impl Default for RelationshipStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// A user's relationship with another user
///
/// If a relationship already exists, user two cannot attempt to create a relationship with user one.
/// The existing relation should be used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// The first user in the relationship
    pub one: Profile,
    /// The second user in the relationship
    pub two: Profile,
    /// The status of the relationship
    pub status: RelationshipStatus,
    /// The timestamp of the relationship's creation
    pub timestamp: u128,
}

/// An IP-based block
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IpBlock {
    /// The ID of the block
    pub id: String,
    /// The IP that was blocked
    pub ip: String,
    /// The user that blocked this IP
    pub user: String,
    /// The context of this block (question content, etc.)
    pub context: String,
    /// The timestamp of when the block was created
    pub timestamp: u128,
}

pub use crate::permissions::FinePermission;

/// Basic permission group
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub name: String,
    pub id: i32,
    pub permissions: FinePermission,
}

impl Default for Group {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            id: 0,
            permissions: FinePermission::default(),
        }
    }
}

/// Mail state
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum MailState {
    /// The mail has been sent, but has never been opened by the recipient
    Unread,
    /// The mail has been opened by the recipient at least once
    Read,
}

/// Basic mail structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Mail {
    /// The title of the mail
    pub title: String,
    /// The content of the mail
    pub content: String,
    /// The timestamp of when the mail was created
    pub timestamp: u128,
    /// The ID of the mail
    pub id: String,
    /// The state of the mail
    pub state: MailState,
    /// The author of the mail
    pub author: String,
    /// The recipient(s) of the mail
    pub recipient: Vec<String>,
}

/// A label which describes a user
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserLabel {
    /// The ID of the label (unique)
    pub id: String,
    /// The name of the label
    pub name: String,
    /// The timestamp of when the label was created
    pub timestamp: u128,
    /// The ID creator of the label
    pub creator: String,
}

/// A coin transaction between two users
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    /// The ID of the transaction (unique)
    pub id: String,
    /// The amount of the transaction
    pub amount: i32,
    /// The ID of the item purchased
    pub item: String,
    /// The timestamp of when the transaction was created
    pub timestamp: u128,
    /// The ID of the customer (who bought the item)
    pub customer: String,
    /// The ID of the merchant (who sold the item)
    pub merchant: String,
}

/// A marketplace item type
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ItemType {
    Text,
    UserTheme,
}

impl Default for ItemType {
    fn default() -> Self {
        Self::Text
    }
}

impl ToString for ItemType {
    fn to_string(&self) -> String {
        match self {
            ItemType::Text => "Text".to_string(),
            ItemType::UserTheme => "UserTheme".to_string(),
        }
    }
}

/// A marketplace item status
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ItemStatus {
    /// The item has been reviewed by a site moderator and rejected
    Rejected,
    /// The item has not been approved by a site moderator
    Pending,
    /// The item has been approved by a site moderator
    Approved,
    /// The item has been featured by a site moderator
    Featured,
}

impl Default for ItemStatus {
    fn default() -> Self {
        Self::Approved
    }
}

impl ToString for ItemStatus {
    fn to_string(&self) -> String {
        match self {
            ItemStatus::Rejected => "Rejected".to_string(),
            ItemStatus::Pending => "Pending".to_string(),
            ItemStatus::Approved => "Approved".to_string(),
            ItemStatus::Featured => "Featured".to_string(),
        }
    }
}

/// A marketplace item (for [`Transaction`]s)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Item {
    /// The ID of the item (unique)
    pub id: String,
    /// The name of this item
    pub name: String,
    /// The description of this item
    pub description: String,
    /// The number of user coins this item costs
    ///
    /// 0: free
    /// -1: off-sale
    pub cost: i32,
    /// The content of this item
    pub content: String,
    /// The type of this item
    pub r#type: ItemType,
    /// The status of this item
    pub status: ItemStatus,
    /// The timestamp of when the item was created
    pub timestamp: u128,
    /// The ID of the item creator
    pub creator: String,
}

// props
#[derive(Serialize, Deserialize, Debug, Hcaptcha)]
pub struct ProfileCreate {
    pub username: String,
    pub password: String,
    pub policy_consent: bool,
    #[captcha]
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Hcaptcha)]
pub struct ProfileLogin {
    pub username: String,
    pub password: String,
    #[captcha]
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileMetadata {
    pub metadata: ProfileMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileBadges {
    pub badges: Vec<(String, String, String)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileLabels {
    pub labels: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileLinks {
    pub links: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileLayout {
    pub layout: LayoutComponent,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RenderLayout {
    pub layout: LayoutComponent,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileGroup {
    pub group: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileTier {
    pub tier: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileCoins {
    pub coins: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfilePassword {
    pub password: String,
    pub new_password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetProfileUsername {
    pub password: String,
    pub new_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotificationCreate {
    pub title: String,
    pub content: String,
    pub address: String,
    pub recipient: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WarningCreate {
    pub content: String,
    pub recipient: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IpBanCreate {
    pub ip: String,
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IpBlockCreate {
    pub ip: String,
    pub context: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MailCreate {
    pub title: String,
    pub content: String,
    pub recipient: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetMailState {
    pub state: MailState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionCreate {
    // pub customer: String,
    pub merchant: String,
    pub item: String,
    pub amount: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemCreate {
    pub name: String,
    pub description: String,
    pub content: String,
    pub cost: i32,
    pub r#type: ItemType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemEdit {
    pub name: String,
    pub description: String,
    pub cost: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemEditContent {
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetItemStatus {
    pub status: ItemStatus,
}

/// General API errors
#[derive(Debug)]
pub enum DatabaseError {
    TooExpensive,
    MustBeUnique,
    OutOfScope,
    NotAllowed,
    ValueError,
    NotFound,
    TooLong,
    Other,
}

impl DatabaseError {
    pub fn to_string(&self) -> String {
        use DatabaseError::*;
        match self {
            TooExpensive => String::from("You cannot afford to do this. (TooExpensive)"),
            MustBeUnique => String::from("One of the given values must be unique. (MustBeUnique)"),
            OutOfScope => String::from(
                "Cannot generate tokens with permissions the provided token doesn't have. (OutOfScope)",
            ),
            NotAllowed => String::from("You are not allowed to access this resource. (NotAllowed)"),
            ValueError => String::from("One of the field values given is invalid. (ValueError)"),
            NotFound => String::from("No asset with this ID could be found. (NotFound)"),
            TooLong => String::from("Given data is too long. (TooLong)"),
            _ => String::from("An unspecified error has occured"),
        }
    }

    pub fn to_json<T: Default>(&self) -> DefaultReturn<T> {
        DefaultReturn {
            success: false,
            message: self.to_string(),
            payload: T::default(),
        }
    }
}

impl IntoResponse for DatabaseError {
    fn into_response(self) -> Response {
        use crate::model::DatabaseError::*;
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
