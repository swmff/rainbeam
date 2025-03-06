use std::collections::BTreeMap;
use std::str::Split;

use crate::layout::LayoutComponent;
use crate::model::{
    DatabaseError, FinePermission, IpBan, IpBanCreate, IpBlock, IpBlockCreate, Item, ItemCreate,
    ItemEdit, ItemEditContent, ItemStatus, ItemType, Mail, MailCreate, MailState, Profile,
    ProfileCreate, ProfileMetadata, RelationshipStatus, TokenContext, Transaction,
    TransactionCreate, UserLabel, Warning, WarningCreate,
};
use crate::model::{Group, Notification, NotificationCreate, UserFollow};
use hcaptcha_no_wasm::Hcaptcha;
use rainbeam_shared::snow::AlmostSnowflake;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

use databeam::{query as sqlquery, utility, prelude::*};
use pathbufd::{PathBufD, pathd};

pub use rainbeam_shared::config::HCaptchaConfig;

pub type Result<T> = std::result::Result<T, DatabaseError>;
use std::sync::LazyLock;

use crate::{cache_sync, from_row, update_profile_count, simplify};

/// Custom keys allowed to be used as metadata options.
pub static ALLOWED_CUSTOM_KEYS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "sparkler:display_name",
        "sparkler:status_note",
        "sparkler:status_emoji",
        "sparkler:limited_friend_requests",
        "sparkler:limited_chats",
        "sparkler:private_profile",
        "sparkler:allow_drawings",
        "sparkler:biography",
        "sparkler:sidebar",
        "sparkler:avatar_url",
        "sparkler:banner_url",
        "sparkler:banner_fit",
        "sparkler:website_theme",
        "sparkler:allow_profile_themes",
        "sparkler:motivational_header",
        "sparkler:warning",
        "sparkler:anonymous_username",
        "sparkler:anonymous_avatar",
        "sparkler:pinned",
        "sparkler:profile_theme",
        "sparkler:desktop_tl_logo",
        "sparkler:custom_css",
        "sparkler:color_surface",
        "sparkler:color_lowered",
        "sparkler:color_super_lowered",
        "sparkler:color_raised",
        "sparkler:color_super_raised",
        "sparkler:color_text",
        "sparkler:color_text_raised",
        "sparkler:color_text_lowered",
        "sparkler:color_link",
        "sparkler:color_primary",
        "sparkler:color_primary_lowered",
        "sparkler:color_primary_raised",
        "sparkler:color_text_primary",
        "sparkler:color_shadow",
        "sparkler:lock_profile",
        "sparkler:disallow_anonymous",
        "sparkler:disallow_anonymous_comments",
        "sparkler:require_account",
        "sparkler:private_social",
        "sparkler:disable_mailbox",
        "sparkler:filter",
        "sparkler:mail_signature",
        "rainbeam:verify_url",
        "rainbeam:verify_code",
        "rainbeam:market_theme_template",
        "rainbeam:nsfw_profile",
        "rainbeam:share_hashtag",
        "rainbeam:authenticated_only",
        "rainbeam:force_default_layout",
        "rainbeam:disallow_response_comments",
        "rainbeam:view_password",
        "rainbeam:do_not_send_global_questions_to_inbox",
    ]
});

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerOptions {
    /// If new registrations are enabled
    #[serde(default)]
    pub registration_enabled: bool,
    /// HCaptcha configuration
    #[serde(default)]
    pub captcha: HCaptchaConfig,
    /// The header to read user IP from
    #[serde(default)]
    pub real_ip_header: Option<String>,
    /// The directory to serve static assets from
    #[serde(default)]
    pub static_dir: PathBufD,
    /// The location of media uploads on the file system
    #[serde(default)]
    pub media_dir: PathBufD,
    /// The origin of the public server (ex: "https://rainbeam.net")
    ///
    /// Used in embeds and links.
    #[serde(default)]
    pub host: String,
    /// The server ID for ID generation
    pub snowflake_server_id: usize,
    /// A list of image hosts that are blocked
    #[serde(default)]
    pub blocked_hosts: Vec<String>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            registration_enabled: true,
            captcha: HCaptchaConfig::default(),
            real_ip_header: Option::None,
            static_dir: PathBufD::default(),
            media_dir: PathBufD::default(),
            host: String::new(),
            snowflake_server_id: 1234567890,
            blocked_hosts: Vec::new(),
        }
    }
}

/// Database connector
#[derive(Clone)]
pub struct Database {
    pub base: StarterDatabase,
    pub config: ServerOptions,
    pub http: HttpClient,
}

impl Database {
    /// Create a new [`Database`]
    pub async fn new(
        database_options: databeam::DatabaseOpts,
        server_options: ServerOptions,
    ) -> Self {
        let base = StarterDatabase::new(database_options).await;

        Self {
            base: base.clone(),
            http: HttpClient::new(),
            config: server_options,
        }
    }

    /// Pull [`databeam::DatabaseOpts`] from env
    pub fn env_options() -> databeam::DatabaseOpts {
        use std::env::var;
        databeam::DatabaseOpts {
            r#type: match var("DB_TYPE") {
                Ok(v) => Option::Some(v),
                Err(_) => Option::None,
            },
            host: match var("DB_HOST") {
                Ok(v) => Option::Some(v),
                Err(_) => Option::None,
            },
            user: var("DB_USER").unwrap_or(String::new()),
            pass: var("DB_PASS").unwrap_or(String::new()),
            name: var("DB_NAME").unwrap_or(String::new()),
        }
    }

    /// Init database
    pub async fn init(&self) {
        // create tables
        let c = &self.base.db.client;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xprofiles\" (
                id                 TEXT,
                username           TEXT,
                password           TEXT,
                tokens             TEXT,
                metadata           TEXT,
                joined             TEXT,
                gid                TEXT,
                salt               TEXT,
                ips                TEXT,
                badges             TEXT,
                tier               TEXT,
                token_context      TEXT,
                labels             TEXT,
                coins              TEXT DEFAULT '0',
                links              TEXT DEFAULT '{}',
                layout             TEXT DEFAULT '{\"json\":\"default.json\"}',
                question_count     TEXT DEFAULT '0',
                response_count     TEXT DEFAULT '0',
                totp               TEXT DEFAULT '',
                recovery_codes     TEXT DEFAULT '[]'
                notification_count TEXT DEFAULT '0',
                inbox_count        TEXT DEFAULT '0'
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xgroups\" (
                name        TEXT,
                id          TEXT,
                permissions INTEGER
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xfollows\" (
                user      TEXT,
                following TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xnotifications\" (
                title     TEXT,
                content   TEXT,
                address   TEXT,
                timestamp TEXT,
                id        TEXT,
                recipient TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xwarnings\" (
                id        TEXT,
                content   TEXT,
                timestamp TEXT,
                recipient TEXT,
                moderator TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xbans\" (
                id        TEXT,
                ip        TEXT,
                reason    TEXT,
                moderator TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xrelationships\" (
                one       TEXT,
                two       TEXT,
                status    TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xipblocks\" (
                id        TEXT,
                ip        TEXT,
                user      TEXT,
                context   TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xmail\" (
                title     TEXT,
                content   TEXT,
                timestamp TEXT,
                id        TEXT,
                state     TEXT,
                author    TEXT,
                recipient TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xlabels\" (
                id        TEXT,
                name      TEXT,
                timestamp TEXT,
                creator   TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            // "xugc_transactions" to not interfere with real money transactions
            "CREATE TABLE IF NOT EXISTS \"xugc_transactions\" (
                id        TEXT,
                amount    TEXT,
                item      TEXT,
                timestamp TEXT,
                customer  TEXT,
                merchant  TEXT
            )",
        )
        .execute(c)
        .await;

        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xugc_items\" (
                id          TEXT,
                name        TEXT,
                description TEXT,
                cost        TEXT,
                content     TEXT,
                type        TEXT,
                status      TEXT,
                timestamp   TEXT,
                creator     TEXT
            )",
        )
        .execute(c)
        .await;
    }

    // util

    /// Collect from `Split<&str>` while cloning everything to make everything owned.
    pub fn collect_split_owned(input: Split<&str>) -> Vec<String> {
        let mut out = Vec::new();

        for section in input {
            out.push(section.to_owned())
        }

        out
    }

    /// Create a moderator audit log entry.
    pub async fn audit(&self, actor_id: String, content: String) -> Result<()> {
        match self
            .create_notification(
                NotificationCreate {
                    title: format!("[{actor_id}](/+u/{actor_id})"),
                    content,
                    address: format!("/+u/{actor_id}"),
                    recipient: "*(audit)".to_string(), // all staff, audit registry
                },
                None,
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(DatabaseError::Other),
        }
    }

    // profiles

    /// Get profile given the `row` data.
    pub async fn gimme_profile(&self, row: BTreeMap<String, String>) -> Result<Box<Profile>> {
        let id = row.get("id").unwrap().to_string();
        Ok(Box::new(Profile {
            id: id.clone(),
            username: from_row!(row->username()),
            password: from_row!(row->password()),
            salt: from_row!(row->salt(); &String::new()),
            tokens: from_row!(row->tokens(json); DatabaseError::ValueError),
            ips: from_row!(row->ips(json); DatabaseError::ValueError),
            token_context: from_row!(row->token_context(json); DatabaseError::ValueError),
            metadata: from_row!(row->metadata(json); DatabaseError::ValueError),
            badges: from_row!(row->badges(json); DatabaseError::ValueError),
            group: from_row!(row->gid(i32); 0),
            joined: from_row!(row->joined(u128); 0),
            tier: from_row!(row->tier(i32); 0),
            labels: Database::collect_split_owned(row.get("labels").unwrap().split(",")),
            coins: from_row!(row->coins(i32); 0),
            links: from_row!(row->links(json); DatabaseError::ValueError),
            layout: from_row!(row->layout(json); DatabaseError::ValueError),
            question_count: from_row!(row->question_count(usize); 0),
            response_count: cache_sync!(
                |row, id| response_count->(update_profile_response_count in self) {1}
            ),
            totp: row.get("totp").unwrap().to_string(),
            recovery_codes: from_row!(row->recovery_codes(json); DatabaseError::ValueError),
            notification_count: from_row!(row->notification_count(usize); 0),
            inbox_count: from_row!(row->inbox_count(usize); 0),
        }))
    }

    // GET
    fn is_digit(&self, input: &str) -> bool {
        for char in input.chars() {
            if !char.is_numeric() {
                return false;
            }
        }

        true
    }
    /// Fetch a profile correctly.
    pub async fn get_profile(&self, mut id: String) -> Result<Box<Profile>> {
        if id.starts_with("ANSWERED:") {
            // we use the "ANSWERED" prefix whenever we answer a question so it doesn't show up in inboxes
            id = id.replace("ANSWERED:", "");
        }

        if id == "@" {
            return Ok(Box::new(Profile::global()));
        } else if id.starts_with("anonymous#") | (id == "anonymous") | (id == "#") {
            let tag = Profile::anonymous_tag(&id);
            return Ok(Box::new(Profile::anonymous(tag.3)));
        } else if (id == "0") | (id == "system") {
            return Ok(Box::new(Profile::system()));
        }

        // remove circle tag
        if id.contains("%") {
            id = id
                .split("%")
                .collect::<Vec<&str>>()
                .get(0)
                .unwrap()
                .to_string();
        }

        // handle legacy IDs (usernames)
        if (id.len() <= 32) && (!self.is_digit(&id) | (id.len() < 18)) {
            return match self.get_profile_by_username(id).await {
                Ok(ua) => Ok(ua),
                Err(e) => return Err(e),
            };
        }

        match self.get_profile_by_id(id).await {
            Ok(ua) => Ok(ua),
            Err(e) => return Err(e),
        }
    }

    /// Get a [`Profile`] by their hashed ID.
    ///
    /// # Arguments:
    /// * `hashed` - `String` of the profile's hashed ID
    pub async fn get_profile_by_hashed(&self, hashed: String) -> Result<Box<Profile>> {
        // fetch from database
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xprofiles\" WHERE \"tokens\" LIKE ?"
        } else {
            "SELECT * FROM \"xprofiles\" WHERE \"tokens\" LIKE $1"
        };

        let c = &self.base.db.client;
        let row = match sqlquery(query)
            .bind::<&String>(&format!("%\"{hashed}\"%"))
            .fetch_one(c)
            .await
        {
            Ok(u) => self.base.textify_row(u).0,
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(match self.gimme_profile(row).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        })
    }

    /// Get a user by their unhashed ID (hashes ID and then calls [`Database::get_profile_by_hashed()`]).
    ///
    /// # Arguments:
    /// * `unhashed` - `String` of the user's unhashed ID
    pub async fn get_profile_by_unhashed(&self, unhashed: String) -> Result<Box<Profile>> {
        self.get_profile_by_hashed(utility::hash(unhashed.clone()))
            .await
    }

    /// Get a [`Profile`] by their IP.
    ///
    /// # Arguments:
    /// * `hashed` - `String` of the profile's IP
    pub async fn get_profile_by_ip(&self, ip: String) -> Result<Box<Profile>> {
        // fetch from database
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xprofiles\" WHERE \"ips\" LIKE ?"
        } else {
            "SELECT * FROM \"xprofiles\" WHERE \"ips\" LIKE $1"
        };

        let c = &self.base.db.client;
        let row = match sqlquery(query)
            .bind::<&String>(&format!("%\"{ip}\"%"))
            .fetch_one(c)
            .await
        {
            Ok(u) => self.base.textify_row(u).0,
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(match self.gimme_profile(row).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        })
    }

    /// Get a user by their username.
    ///
    /// # Arguments:
    /// * `username` - `String` of the user's username
    pub async fn get_profile_by_username(&self, mut username: String) -> Result<Box<Profile>> {
        username = username.to_lowercase();

        // check in cache
        let cached = self
            .base
            .cachedb
            .get(format!("rbeam.auth.profile:{}", username))
            .await;

        if cached.is_some() {
            match serde_json::from_str::<Profile>(cached.unwrap().as_str()) {
                Ok(p) => return Ok(Box::new(p)),
                Err(_) => {
                    self.base
                        .cachedb
                        .remove(format!("rbeam.auth.profile:{}", username))
                        .await;
                }
            };
        }

        // ...
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xprofiles\" WHERE \"username\" = ?"
        } else {
            "SELECT * FROM \"xprofiles\" WHERE \"username\" = $1"
        };

        let c = &self.base.db.client;
        let row = match sqlquery(query)
            .bind::<&String>(&username)
            .fetch_one(c)
            .await
        {
            Ok(r) => self.base.textify_row(r).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // store in cache
        let user = match self.gimme_profile(row).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        self.base
            .cachedb
            .set(
                format!("rbeam.auth.profile:{}", username),
                serde_json::to_string::<Profile>(&user).unwrap(),
            )
            .await;

        // return
        Ok(user)
    }

    /// Get a user by their id.
    ///
    /// # Arguments:
    /// * `id` - `String` of the user's username
    pub async fn get_profile_by_id(&self, mut id: String) -> Result<Box<Profile>> {
        id = id.to_lowercase();

        // check in cache
        let cached = self
            .base
            .cachedb
            .get(format!("rbeam.auth.profile:{}", id))
            .await;

        if cached.is_some() {
            match serde_json::from_str::<Profile>(cached.unwrap().as_str()) {
                Ok(p) => return Ok(Box::new(p)),
                Err(_) => {
                    self.base
                        .cachedb
                        .remove(format!("rbeam.auth.profile:{}", id))
                        .await;
                }
            };
        }

        // ...
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xprofiles\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xprofiles\" WHERE \"id\" = $1"
        };

        let c = &self.base.db.client;
        let row = match sqlquery(query).bind::<&String>(&id).fetch_one(c).await {
            Ok(r) => self.base.textify_row(r).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // store in cache
        let user = match self.gimme_profile(row).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        self.base
            .cachedb
            .set(
                format!("rbeam.auth.profile:{}", id),
                serde_json::to_string::<Profile>(&user).unwrap(),
            )
            .await;

        // return
        Ok(user)
    }

    /// Validate a username.
    pub fn validate_username(username: String) -> Result<()> {
        let banned_usernames = &[
            "admin",
            "account",
            "anonymous",
            "login",
            "sign_up",
            "settings",
            "api",
            "intents",
            "circles",
            "chats",
            "sites",
            "responses",
            "questions",
            "comments",
            "response",
            "question",
            "comment",
            "pages",
            "inbox",
            "system",
            "market",
            ".well-known",
            "static",
        ];

        let regex = regex::RegexBuilder::new(r"[^\w_\-\.!]+")
            .multi_line(true)
            .build()
            .unwrap();

        if regex.captures(&username).is_some() {
            return Err(DatabaseError::ValueError);
        }

        if (username.len() < 2) | (username.len() > 500) {
            return Err(DatabaseError::ValueError);
        }

        if banned_usernames.contains(&username.as_str()) {
            return Err(DatabaseError::ValueError);
        }

        Ok(())
    }

    // SET
    /// Create a new user given their username. Returns their unhashed token.
    ///
    /// # Arguments:
    /// * `username` - `String` of the user's `username`
    /// * `user_ip` - the ip address of the user registering
    pub async fn create_profile(&self, props: ProfileCreate, user_ip: String) -> Result<String> {
        if self.config.registration_enabled == false {
            return Err(DatabaseError::NotAllowed);
        }

        // ...
        let username = props.username.trim().to_string();
        let password = props.password.trim().to_string();

        // check captcha
        if let Err(_) = props
            .valid_response(&self.config.captcha.secret, None)
            .await
        {
            return Err(DatabaseError::NotAllowed);
        }

        // make sure user doesn't already exists
        if let Ok(_) = &self.get_profile_by_username(username.clone()).await {
            return Err(DatabaseError::MustBeUnique);
        };

        // check username
        if let Err(e) = Database::validate_username(username.clone()) {
            return Err(e);
        }

        // ...
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "INSERT INTO \"xprofiles\" VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xprofiles\" VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)"
        };

        let user_token_unhashed: String = databeam::utility::uuid();
        let user_token_hashed: String = databeam::utility::hash(user_token_unhashed.clone());
        let salt: String = rainbeam_shared::hash::salt();

        let timestamp = utility::unix_epoch_timestamp().to_string();

        let c = &self.base.db.client;
        match sqlquery(query)
            // .bind::<&String>(&databeam::utility::uuid())
            .bind::<&String>(&AlmostSnowflake::new(self.config.snowflake_server_id).to_string())
            .bind::<&String>(&username.to_lowercase())
            .bind::<&String>(&rainbeam_shared::hash::hash_salted(password, salt.clone()))
            .bind::<&String>(
                &serde_json::to_string::<Vec<String>>(&vec![user_token_hashed]).unwrap(),
            )
            .bind::<&String>(
                &serde_json::to_string::<ProfileMetadata>(&ProfileMetadata::default()).unwrap(),
            )
            .bind::<&String>(&timestamp)
            .bind::<i8>(0)
            .bind::<&String>(&salt)
            .bind::<&String>(&serde_json::to_string::<Vec<String>>(&vec![user_ip]).unwrap())
            .bind::<&str>("[]")
            .bind::<i8>(0)
            .bind::<&str>("[]")
            .bind::<&str>("")
            .bind::<i8>(0) // start with 100 coins
            .bind::<&str>("{}")
            .bind::<&str>("{\"json\":\"default.json\"}")
            .bind::<i8>(0)
            .bind::<i8>(0)
            .bind::<&str>("")
            .bind::<&str>("[]")
            .bind::<i8>(0)
            .bind::<i8>(0)
            .execute(c)
            .await
        {
            Ok(_) => Ok(user_token_unhashed),
            Err(_) => Err(DatabaseError::Other),
        }
    }

    update_profile_count!(update_profile_question_count, question_count);
    update_profile_count!(update_profile_response_count, response_count);
    update_profile_count!(update_profile_inbox_count, inbox_count);
    update_profile_count!(update_profile_notification_count, notification_count);

    /// Update a [`Profile`]'s metadata by its `id`.
    pub async fn update_profile_metadata(
        &self,
        id: String,
        mut metadata: ProfileMetadata,
    ) -> Result<()> {
        // make sure user exists
        let profile = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // check metadata kv
        for kv in metadata.kv.clone() {
            if !ALLOWED_CUSTOM_KEYS.contains(&kv.0.as_str()) {
                metadata.kv.remove(&kv.0);
            }
        }

        if !metadata.check() {
            return Err(DatabaseError::TooLong);
        }

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"metadata\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"metadata\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        let meta = &serde_json::to_string(&metadata).unwrap();
        match sqlquery(query)
            .bind::<&String>(meta)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", profile.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", profile.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s tokens (and IPs/token_contexts) by its `id`
    pub async fn update_profile_tokens(
        &self,
        id: String,
        tokens: Vec<String>,
        ips: Vec<String>,
        token_context: Vec<TokenContext>,
    ) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"tokens\" = ?, \"ips\" = ?, \"token_context\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"tokens\", \"ips\") = ($1, $2, $3) WHERE \"id\" = $4"
        };

        let c = &self.base.db.client;

        let tokens = &serde_json::to_string(&tokens).unwrap();
        let ips = &serde_json::to_string(&ips).unwrap();
        let token_context = &serde_json::to_string(&token_context).unwrap();

        match sqlquery(query)
            .bind::<&String>(tokens)
            .bind::<&String>(ips)
            .bind::<&String>(token_context)
            .bind::<&String>(&ua.id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s badges by its `id`
    pub async fn update_profile_badges(
        &self,
        id: String,
        badges: Vec<(String, String, String)>,
    ) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"badges\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"badges\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        let badges = &serde_json::to_string(&badges).unwrap();

        match sqlquery(query)
            .bind::<&String>(badges)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s labels by its `id`
    pub async fn update_profile_labels(&self, id: String, labels: Vec<String>) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"labels\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"labels\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        let labels = &serde_json::to_string(&labels).unwrap();

        match sqlquery(query)
            .bind::<&String>(labels)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s links by its `id`
    pub async fn update_profile_links(
        &self,
        id: String,
        links: BTreeMap<String, String>,
    ) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"links\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"links\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        let links = &serde_json::to_string(&links).unwrap();

        match sqlquery(query)
            .bind::<&String>(links)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s links by its `id`
    pub async fn update_profile_layout(&self, id: String, layout: LayoutComponent) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"layout\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"layout\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        let layout = &serde_json::to_string(&layout).unwrap();

        match sqlquery(query)
            .bind::<&String>(layout)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s tier by its ID
    pub async fn update_profile_tier(&self, id: String, tier: i32) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"tier\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"tier\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&tier.to_string())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s `gid` by its `id`
    pub async fn update_profile_group(&self, id: String, group: i32) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"gid\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"gid\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&i32>(&group)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s coins by its ID
    ///
    /// # Arguments
    /// * `coins` - the amount to ADD to the existing coins value
    pub async fn update_profile_coins(&self, id: String, coins: i32) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"coins\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"coins\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&(ua.coins + coins).to_string())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s `password` by its name and password
    pub async fn update_profile_password(
        &self,
        id: String,
        password: String,
        new_password: String,
        do_password_check: bool,
    ) -> Result<()> {
        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // check password
        if do_password_check {
            let password_hashed = rainbeam_shared::hash::hash_salted(password, ua.salt);

            if password_hashed != ua.password {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"password\" = ?, \"salt\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"password\", \"salt\") = ($1, $2) WHERE \"id\" = $3"
        };

        let new_salt = rainbeam_shared::hash::salt();

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&rainbeam_shared::hash::hash_salted(
                new_password,
                new_salt.clone(),
            ))
            .bind::<&String>(&new_salt)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update a [`Profile`]'s `username` by its id and password
    pub async fn update_profile_username(
        &self,
        id: String,
        password: String,
        mut new_name: String,
    ) -> Result<()> {
        new_name = new_name.to_lowercase();

        // make sure user exists
        let ua = match self.get_profile(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // make sure username isn't in use
        if let Ok(_) = self.get_profile_by_username(new_name.clone()).await {
            return Err(DatabaseError::MustBeUnique);
        }

        // check username
        if let Err(e) = Database::validate_username(new_name.clone()) {
            return Err(e);
        }

        // check password
        let password_hashed = rainbeam_shared::hash::hash_salted(password, ua.salt);

        if password_hashed != ua.password {
            return Err(DatabaseError::NotAllowed);
        }

        // update user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "UPDATE \"xprofiles\" SET \"username\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"username\") = ($1) WHERE \"id\" = $2"
        };

        let c = &self.base.db.client;
        match sqlquery(query)
            .bind::<&String>(&new_name)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.id))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Delete a profile
    ///
    /// **VALIDATION SHOULD BE DONE *BEFORE* THIS!!**
    async fn delete_profile(&self, id: String) -> Result<()> {
        let user = self.get_profile_by_id(id.clone()).await.unwrap();

        // delete user
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "DELETE FROM \"xprofiles\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xprofiles\" WHERE \"id\" = $1"
        };

        let c = &self.base.db.client;
        match sqlquery(query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xnotifications\" WHERE \"recipient\" = ?"
                    } else {
                        "DELETE FROM \"xnotifications\" WHERE \"recipient\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xwarnings\" WHERE \"recipient\" = ?"
                    } else {
                        "DELETE FROM \"xwarnings\" WHERE \"recipient\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xfollows\" WHERE \"user\" = ? OR \"following\" = ?"
                    } else {
                        "DELETE FROM \"xfollows\" WHERE \"user\" = $1 OR \"following\" = $2"
                    };

                if let Err(_) = sqlquery(query)
                    .bind::<&String>(&id)
                    .bind::<&String>(&id)
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                // rainbeam crate stuff
                // questions to user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xquestions\" WHERE \"recipient\" = ?"
                    } else {
                        "DELETE FROM \"xquestions\" WHERE \"recipient\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                // questions by user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xquestions\" WHERE \"author\" = ?"
                    } else {
                        "DELETE FROM \"xquestions\" WHERE \"author\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                // responses by user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xresponses\" WHERE \"author\" = ?"
                    } else {
                        "DELETE FROM \"xresponses\" WHERE \"author\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                // responses to questions by user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xresponses\" WHERE \"question\" LIKE ?"
                    } else {
                        "DELETE FROM \"xresponses\" WHERE \"question\" LIKE $1"
                    };

                if let Err(_) = sqlquery(query)
                    .bind::<&String>(&format!("%\"author\":\"{id}\"%"))
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                self.base
                    .cachedb
                    .remove(format!("rbeam.app.response_count:{}", id))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.app.global_question_count:{}", id))
                    .await;

                // circles by user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xcircles\" WHERE \"owner\" = ?"
                    } else {
                        "DELETE FROM \"xcircles\" WHERE \"owner\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                // user circle memberships
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xcircle_memberships\" WHERE \"user\" = ?"
                    } else {
                        "DELETE FROM \"xcircle_memberships\" WHERE \"user\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                // relationships involving user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xrelationships\" WHERE \"one\" = ? OR \"two\" = ?"
                    } else {
                        "DELETE FROM \"xrelationships\" WHERE \"one\" = $1 OR \"two\" = $2"
                    };

                if let Err(_) = sqlquery(query)
                    .bind::<&String>(&id)
                    .bind::<&String>(&id)
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                self.base
                    .cachedb
                    .remove(format!("rbeam.app.friends_count:{}", id))
                    .await;

                // ipblocks by user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xipblocks\" WHERE \"user\" = ?"
                    } else {
                        "DELETE FROM \"xipblocks\" WHERE \"user\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                // transactions with user
                let query: &str = if (self.base.db.r#type == "sqlite")
                    | (self.base.db.r#type == "mysql")
                {
                    "DELETE FROM \"xugc_transactions\" WHERE \"customer\" = ? OR \"merchant\" = ?"
                } else {
                    "DELETE FROM \"xugc_transactions\" WHERE \"customer\" = $1 OR \"merchant\" = $2"
                };

                if let Err(_) = sqlquery(query)
                    .bind::<&String>(&id)
                    .bind::<&String>(&id)
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                // items by user
                let query: &str =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xugc_items\" WHERE \"creator\" = ?"
                    } else {
                        "DELETE FROM \"xugc_items\" WHERE \"creator\" = $1"
                    };

                if let Err(_) = sqlquery(query).bind::<&String>(&id).execute(c).await {
                    return Err(DatabaseError::Other);
                };

                // ...
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", id))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", user.username))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.followers_count:{}", id))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.following_count:{}", id))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.notification_count:{}", id))
                    .await;

                // delete images
                // TODO: Implement `.is_empty()` for `PathBufD`
                if !self.config.media_dir.to_string().is_empty() {
                    let avatar = pathd!("{}/avatars/{}.avif", self.config.media_dir, id);
                    if let Ok(_) = rainbeam_shared::fs::fstat(&avatar) {
                        if let Err(_) = rainbeam_shared::fs::remove_file(avatar) {
                            return Err(DatabaseError::Other);
                        }
                    }

                    let banner = pathd!("{}/banners/{}.avif", self.config.media_dir, id);
                    if let Ok(_) = rainbeam_shared::fs::fstat(&banner) {
                        if let Err(_) = rainbeam_shared::fs::remove_file(banner) {
                            return Err(DatabaseError::Other);
                        }
                    }
                }

                // ...
                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Delete an existing [`Profile`] by its `id`
    pub async fn delete_profile_by_id(&self, id: String) -> Result<()> {
        let user = match self.get_profile_by_id(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // make sure they aren't a manager
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if group.permissions.check(FinePermission::DELETE_USER) {
            return Err(DatabaseError::NotAllowed);
        }

        // delete
        self.delete_profile(id).await
    }

    // groups

    // GET
    /// Get a group by its id
    ///
    /// # Arguments:
    /// * `username` - `String` of the user's username
    pub async fn get_group_by_id(&self, id: i32) -> Result<Group> {
        // check in cache
        let cached = self
            .base
            .cachedb
            .get(format!("rbeam.auth.gid:{}", id))
            .await;

        if cached.is_some() {
            return Ok(serde_json::from_str::<Group>(cached.unwrap().as_str()).unwrap());
        }

        // ...
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xgroups\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xgroups\" WHERE \"id\" = $1"
        };

        let c = &self.base.db.client;
        let row = match sqlquery(query).bind::<&i32>(&id).fetch_one(c).await {
            Ok(r) => self.base.textify_row(r).0,
            Err(_) => return Ok(Group::default()),
        };

        // store in cache
        let group = Group {
            name: row.get("name").unwrap().to_string(),
            id: row.get("id").unwrap().parse::<i32>().unwrap(),
            permissions: match serde_json::from_str(row.get("permissions").unwrap()) {
                Ok(m) => m,
                Err(_) => return Err(DatabaseError::ValueError),
            },
        };

        self.base
            .cachedb
            .set(
                format!("rbeam.auth.gid:{}", id),
                serde_json::to_string::<Group>(&group).unwrap(),
            )
            .await;

        // return
        Ok(group)
    }

    // profiles

    // GET
    /// Get an existing [`UserFollow`]
    ///
    /// # Arguments:
    /// * `user`
    /// * `following`
    pub async fn get_follow(&self, user: String, following: String) -> Result<UserFollow> {
        // fetch from database
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xfollows\" WHERE \"user\" = ? AND \"following\" = ?"
        } else {
            "SELECT * FROM \"xfollows\" WHERE \"user\" = $1 AND \"following\" = $2"
        };

        let c = &self.base.db.client;
        let row = match sqlquery(query)
            .bind::<&String>(&user)
            .bind::<&String>(&following)
            .fetch_one(c)
            .await
        {
            Ok(u) => self.base.textify_row(u).0,
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(UserFollow {
            user: row.get("user").unwrap().to_string(),
            following: row.get("following").unwrap().to_string(),
        })
    }

    /// Get all existing [`UserFollow`]s where `following` is the value of `user`
    ///
    /// # Arguments:
    /// * `user`
    pub async fn get_followers(
        &self,
        user: String,
    ) -> Result<Vec<(UserFollow, Box<Profile>, Box<Profile>)>> {
        // fetch from database
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xfollows\" WHERE \"following\" = ?"
        } else {
            "SELECT * FROM \"xfollows\" WHERE \"following\" = $1"
        };

        let c = &self.base.db.client;
        let res = match sqlquery(query).bind::<&String>(&user).fetch_all(c).await {
            Ok(u) => {
                let mut out = Vec::new();

                for row in u {
                    let row = self.base.textify_row(row).0;

                    let user = row.get("user").unwrap().to_string();
                    let following = row.get("following").unwrap().to_string();

                    out.push((
                        UserFollow {
                            user: user.clone(),
                            following: following.clone(),
                        },
                        match self.get_profile_by_id(user).await {
                            Ok(ua) => ua,
                            Err(e) => return Err(e),
                        },
                        match self.get_profile_by_id(following).await {
                            Ok(ua) => ua,
                            Err(e) => return Err(e),
                        },
                    ))
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all existing [`UserFollow`]s where `following` is the value of `user`, 12 at a time
    ///
    /// # Arguments:
    /// * `user`
    /// * `page`
    pub async fn get_followers_paginated(
        &self,
        user: String,
        page: i32,
    ) -> Result<Vec<(UserFollow, Box<Profile>, Box<Profile>)>> {
        // fetch from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!(
                "SELECT * FROM \"xfollows\" WHERE \"following\" = ? LIMIT 12 OFFSET {}",
                page * 12
            )
        } else {
            format!(
                "SELECT * FROM \"xfollows\" WHERE \"following\" = $1 LIMIT 12 OFFSET {}",
                page * 12
            )
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(u) => {
                let mut out = Vec::new();

                for row in u {
                    let row = self.base.textify_row(row).0;

                    let user = row.get("user").unwrap().to_string();
                    let following = row.get("following").unwrap().to_string();

                    out.push((
                        UserFollow {
                            user: user.clone(),
                            following: following.clone(),
                        },
                        match self.get_profile_by_id(user.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) UID 'user' {}", e.to_string(), user);

                                continue;
                            }
                        },
                        match self.get_profile_by_id(following.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) UID 'following' {}", e.to_string(), following);

                                continue;
                            }
                        },
                    ))
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get the number of followers `user` has
    ///
    /// # Arguments:
    /// * `user`
    pub async fn get_followers_count(&self, user: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("rbeam.auth.followers_count:{}", user))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_followers(user.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("rbeam.auth.followers_count:{}", user),
                count.to_string(),
            )
            .await;

        count
    }

    /// Get all existing [`UserFollow`]s where `user` is the value of `user`
    ///
    /// # Arguments:
    /// * `user`
    pub async fn get_following(
        &self,
        user: String,
    ) -> Result<Vec<(UserFollow, Box<Profile>, Box<Profile>)>> {
        // fetch from database
        let query: &str = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
            "SELECT * FROM \"xfollows\" WHERE \"user\" = ?"
        } else {
            "SELECT * FROM \"xfollows\" WHERE \"user\" = $1"
        };

        let c = &self.base.db.client;
        let res = match sqlquery(query).bind::<&String>(&user).fetch_all(c).await {
            Ok(u) => {
                let mut out = Vec::new();

                for row in u {
                    let row = self.base.textify_row(row).0;

                    let user = row.get("user").unwrap().to_string();
                    let following = row.get("following").unwrap().to_string();

                    out.push((
                        UserFollow {
                            user: user.clone(),
                            following: following.clone(),
                        },
                        match self.get_profile_by_id(user.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) UID 'user' {}", e.to_string(), user);

                                continue;
                            }
                        },
                        match self.get_profile_by_id(following.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) UID 'following' {}", e.to_string(), following);

                                continue;
                            }
                        },
                    ))
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all existing [`UserFollow`]s where `user` is the value of `user`, 12 at a time
    ///
    /// # Arguments:
    /// * `user`
    /// * `page`
    pub async fn get_following_paginated(
        &self,
        user: String,
        page: i32,
    ) -> Result<Vec<(UserFollow, Box<Profile>, Box<Profile>)>> {
        // fetch from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!(
                "SELECT * FROM \"xfollows\" WHERE \"user\" = ? LIMIT 12 OFFSET {}",
                page * 12
            )
        } else {
            format!(
                "SELECT * FROM \"xfollows\" WHERE \"user\" = $1 LIMIT 12 OFFSET {}",
                page * 12
            )
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(u) => {
                let mut out = Vec::new();

                for row in u {
                    let row = self.base.textify_row(row).0;

                    let user = row.get("user").unwrap().to_string();
                    let following = row.get("following").unwrap().to_string();

                    out.push((
                        UserFollow {
                            user: user.clone(),
                            following: following.clone(),
                        },
                        match self.get_profile_by_id(user.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) UID 'user' {}", e.to_string(), user);

                                continue;
                            }
                        },
                        match self.get_profile_by_id(following.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) UID 'following' {}", e.to_string(), following);

                                continue;
                            }
                        },
                    ))
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get the number of users `user` is following
    ///
    /// # Arguments:
    /// * `user`
    pub async fn get_following_count(&self, user: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("rbeam.auth.following_count:{}", user))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_following(user.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("rbeam.auth.following_count:{}", user),
                count.to_string(),
            )
            .await;

        count
    }

    // SET
    /// Toggle the following status of `user` on `following` ([`UserFollow`])
    ///
    /// # Arguments:
    /// * `props` - [`UserFollow`]
    pub async fn toggle_user_follow(&self, props: &mut UserFollow) -> Result<()> {
        // users cannot be the same
        if props.user == props.following {
            return Err(DatabaseError::Other);
        }

        // make sure both users exist
        let user_1 = match self.get_profile(props.user.to_owned()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // make sure both users exist
        if let Err(e) = self.get_profile(props.following.to_owned()).await {
            return Err(e);
        };

        // check if follow exists
        if let Ok(_) = self
            .get_follow(props.user.to_owned(), props.following.to_owned())
            .await
        {
            // delete
            let query: String =
                if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                    "DELETE FROM \"xfollows\" WHERE \"user\" = ? AND \"following\" = ?"
                } else {
                    "DELETE FROM \"xfollows\" WHERE \"user\" = $1 AND \"following\" = $2"
                }
                .to_string();

            let c = &self.base.db.client;
            match sqlquery(&query)
                .bind::<&String>(&props.user)
                .bind::<&String>(&props.following)
                .execute(c)
                .await
            {
                Ok(_) => {
                    self.base
                        .cachedb
                        .decr(format!("rbeam.auth.following_count:{}", props.user))
                        .await;

                    self.base
                        .cachedb
                        .decr(format!("rbeam.auth.followers_count:{}", props.following))
                        .await;

                    return Ok(());
                }
                Err(_) => return Err(DatabaseError::Other),
            };
        }

        // return
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xfollows\" VALUES (?, ?)"
        } else {
            "INSERT INTO \"xfollows\" VALEUS ($1, $2)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&props.user)
            .bind::<&String>(&props.following)
            .execute(c)
            .await
        {
            Ok(_) => {
                // bump counts
                self.base
                    .cachedb
                    .incr(format!("rbeam.auth.following_count:{}", props.user))
                    .await;

                self.base
                    .cachedb
                    .incr(format!("rbeam.auth.followers_count:{}", props.following))
                    .await;

                // create notification
                if let Err(e) = self
                    .create_notification(
                        NotificationCreate {
                            title: format!(
                                "[@{}](/+u/{}) followed you!",
                                user_1.username, user_1.id
                            ),
                            content: String::new(),
                            address: format!("/+u/{}", user_1.id),
                            recipient: props.following.clone(),
                        },
                        None,
                    )
                    .await
                {
                    return Err(e);
                };

                // return
                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Force remove the following status of `user` on `following` ([`UserFollow`])
    ///
    /// # Arguments:
    /// * `props` - [`UserFollow`]
    pub async fn force_remove_user_follow(&self, props: &mut UserFollow) -> Result<()> {
        // users cannot be the same
        if props.user == props.following {
            return Err(DatabaseError::Other);
        }

        // check if follow exists
        if let Ok(_) = self
            .get_follow(props.user.to_owned(), props.following.to_owned())
            .await
        {
            // delete
            let query: String =
                if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                    "DELETE FROM \"xfollows\" WHERE \"user\" = ? AND \"following\" = ?"
                } else {
                    "DELETE FROM \"xfollows\" WHERE \"user\" = $1 AND \"following\" = $2"
                }
                .to_string();

            let c = &self.base.db.client;
            match sqlquery(&query)
                .bind::<&String>(&props.user)
                .bind::<&String>(&props.following)
                .execute(c)
                .await
            {
                Ok(_) => {
                    self.base
                        .cachedb
                        .decr(format!("rbeam.auth.following_count:{}", props.user))
                        .await;

                    self.base
                        .cachedb
                        .decr(format!("rbeam.auth.followers_count:{}", props.following))
                        .await;

                    return Ok(());
                }
                Err(_) => return Err(DatabaseError::Other),
            };
        }

        // return
        // we can only remove following here, not add it
        Ok(())
    }

    // notifications

    // GET
    /// Get an existing notification
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_notification(&self, id: String) -> Result<Notification> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.notification:{}", id))
            .await
        {
            Some(c) => return Ok(serde_json::from_str::<Notification>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xnotifications\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xnotifications\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let notification = Notification {
            title: res.get("title").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            address: res.get("address").unwrap().to_string(),
            timestamp: from_row!(res->timestamp(u128); 0),
            id: from_row!(res->id()),
            recipient: res.get("recipient").unwrap().to_string(),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.notification:{}", id),
                serde_json::to_string::<Notification>(&notification).unwrap(),
            )
            .await;

        // return
        Ok(notification)
    }

    /// Get all notifications by their recipient
    ///
    /// # Arguments
    /// * `recipient`
    pub async fn get_notifications_by_recipient(
        &self,
        recipient: String,
    ) -> Result<Vec<Notification>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xnotifications\" WHERE \"recipient\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xnotifications\" WHERE \"recipient\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&recipient.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Notification> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(Notification {
                        title: res.get("title").unwrap().to_string(),
                        content: res.get("content").unwrap().to_string(),
                        address: res.get("address").unwrap().to_string(),
                        timestamp: from_row!(res->timestamp(u128); 0),
                        id: from_row!(res->id()),
                        recipient: res.get("recipient").unwrap().to_string(),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of notifications by their recipient
    ///
    /// # Arguments
    /// * `recipient`
    pub async fn get_notification_count_by_recipient_cache(&self, recipient: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("rbeam.auth.notification_count:{}", recipient))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_notifications_by_recipient(recipient.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("rbeam.auth.notification_count:{}", recipient),
                count.to_string(),
            )
            .await;

        count
    }

    /// Get the number of notifications by their recipient
    ///
    /// # Arguments
    /// * `recipient`
    pub async fn get_notification_count_by_recipient(&self, recipient: String) -> usize {
        match self.get_profile(recipient).await {
            Ok(x) => x.notification_count,
            Err(_) => 0,
        }
    }

    /// Get all notifications by their recipient, 12 at a time
    ///
    /// # Arguments
    /// * `recipient`
    /// * `page`
    pub async fn get_notifications_by_recipient_paginated(
        &self,
        recipient: String,
        page: i32,
    ) -> Result<Vec<Notification>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xnotifications\" WHERE \"recipient\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xnotifications\" WHERE \"recipient\" = $1 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&recipient.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Notification> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(Notification {
                        title: res.get("title").unwrap().to_string(),
                        content: res.get("content").unwrap().to_string(),
                        address: res.get("address").unwrap().to_string(),
                        timestamp: from_row!(res->timestamp(u128); 0),
                        id: from_row!(res->id()),
                        recipient: res.get("recipient").unwrap().to_string(),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    // SET
    /// Create a new notification
    ///
    /// # Arguments
    /// * `props` - [`NotificationCreate`]
    pub async fn create_notification(
        &self,
        props: NotificationCreate,
        id: Option<String>,
    ) -> Result<()> {
        let notification = Notification {
            title: props.title,
            content: props.content,
            address: props.address,
            timestamp: utility::unix_epoch_timestamp(),
            id: if let Some(id) = id {
                id
            } else {
                AlmostSnowflake::new(self.config.snowflake_server_id).to_string()
            },
            recipient: props.recipient,
        };

        // create notification
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xnotifications\" VALUES (?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xnotifications\" VALEUS ($1, $2, $3, $4, $5, $6)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&notification.title)
            .bind::<&String>(&notification.content)
            .bind::<&String>(&notification.address)
            .bind::<&String>(&notification.timestamp.to_string())
            .bind::<&String>(&notification.id)
            .bind::<&String>(&notification.recipient)
            .execute(c)
            .await
        {
            Ok(_) => {
                // incr notifications count
                self.base
                    .cachedb
                    .incr(format!(
                        "rbeam.auth.notification_count:{}",
                        notification.recipient
                    ))
                    .await;

                // check recipient
                if !notification.recipient.starts_with("*") {
                    let recipient =
                        simplify!(self.get_profile(notification.recipient.clone()).await; Result);

                    simplify!(
                        self.update_profile_notification_count(
                            notification.recipient,
                            recipient.notification_count + 1,
                        )
                        .await; Err
                    );
                }

                // ...
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing notification
    ///
    /// Notifications can only be deleted by their recipient.
    ///
    /// # Arguments
    /// * `id` - the ID of the notification
    /// * `user` - the user doing this
    pub async fn delete_notification(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure notification exists
        let notification = match self.get_notification(id.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != notification.recipient {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group
                .permissions
                .check(FinePermission::MANAGE_NOTIFICATIONS)
            {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete notification
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xnotifications\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xnotifications\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // decr notifications count
                self.base
                    .cachedb
                    .decr(format!(
                        "rbeam.auth.notification_count:{}",
                        notification.recipient
                    ))
                    .await;

                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.notification:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete all existing notifications by their recipient
    ///
    /// # Arguments
    /// * `id` - the ID of the notification
    /// * `user` - the user doing this
    pub async fn delete_notifications_by_recipient(
        &self,
        recipient: String,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure notifications exists
        let notifications = match self.get_notifications_by_recipient(recipient.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != recipient {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group
                .permissions
                .check(FinePermission::MANAGE_NOTIFICATIONS)
            {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete notifications
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xnotifications\" WHERE \"recipient\" = ?"
        } else {
            "DELETE FROM \"xnotifications\" WHERE \"recipient\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&recipient)
            .execute(c)
            .await
        {
            Ok(_) => {
                // clear notifications count
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.notification_count:{}", recipient))
                    .await;

                // clear cache for all deleted notifications
                for notification in notifications {
                    // remove from cache
                    self.base
                        .cachedb
                        .remove(format!("rbeam.auth.notification:{}", notification.id))
                        .await;
                }

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // warnings

    // GET
    /// Get an existing warning
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_warning(&self, id: String) -> Result<Warning> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.warning:{}", id))
            .await
        {
            Some(c) => return Ok(serde_json::from_str::<Warning>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xwarnings\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xwarnings\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let warning = Warning {
            id: from_row!(res->id()),
            content: res.get("content").unwrap().to_string(),
            timestamp: from_row!(res->timestamp(u128); 0),
            recipient: res.get("recipient").unwrap().to_string(),
            moderator: match self
                .get_profile_by_id(res.get("moderator").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.warning:{}", id),
                serde_json::to_string::<Warning>(&warning).unwrap(),
            )
            .await;

        // return
        Ok(warning)
    }

    /// Get all warnings by their recipient
    ///
    /// # Arguments
    /// * `recipient`
    /// * `user` - the user doing this
    pub async fn get_warnings_by_recipient(
        &self,
        recipient: String,
        user: Box<Profile>,
    ) -> Result<Vec<Warning>> {
        // make sure user is a manager
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if !group.permissions.check(FinePermission::MANAGE_WARNINGS) {
            return Err(DatabaseError::NotAllowed);
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xwarnings\" WHERE \"recipient\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xwarnings\" WHERE \"recipient\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&recipient.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Warning> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(Warning {
                        id: from_row!(res->id()),
                        content: res.get("content").unwrap().to_string(),
                        timestamp: from_row!(res->timestamp(u128); 0),
                        recipient: res.get("recipient").unwrap().to_string(),
                        moderator: match self
                            .get_profile_by_id(res.get("moderator").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    // SET
    /// Create a new warning
    ///
    /// # Arguments
    /// * `props` - [`WarningCreate`]
    /// * `user` - the user creating this warning
    pub async fn create_warning(&self, props: WarningCreate, user: Box<Profile>) -> Result<()> {
        // make sure user is a manager
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if !group.permissions.check(FinePermission::MANAGE_WARNINGS) {
            return Err(DatabaseError::NotAllowed);
        }

        // ...
        let warning = Warning {
            id: utility::random_id(),
            content: props.content,
            timestamp: utility::unix_epoch_timestamp(),
            recipient: props.recipient,
            moderator: user,
        };

        // create notification
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xwarnings\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xwarnings\" VALEUS ($1, $2, $3, $4, $5)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&warning.id)
            .bind::<&String>(&warning.content)
            .bind::<&String>(&warning.timestamp.to_string())
            .bind::<&String>(&warning.recipient)
            .bind::<&String>(&warning.moderator.id)
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification for recipient
                if let Err(e) = self
                    .create_notification(
                        NotificationCreate {
                            title: "You have received an account warning!".to_string(),
                            content: warning.content,
                            address: String::new(),
                            recipient: warning.recipient,
                        },
                        None,
                    )
                    .await
                {
                    return Err(e);
                };

                // ...
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing warning
    ///
    /// Warnings can only be deleted by their moderator or admins.
    ///
    /// # Arguments
    /// * `id` - the ID of the warning
    /// * `user` - the user doing this
    pub async fn delete_warning(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure warning exists
        let warning = match self.get_warning(id.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check id
        if user.id != warning.moderator.id {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_WARNINGS) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete warning
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xwarnings\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xwarnings\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.warning:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // ip bans

    // GET
    /// Get an existing [`IpBan`]
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_ipban(&self, id: String) -> Result<IpBan> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.ipban:{}", id))
            .await
        {
            Some(c) => return Ok(serde_json::from_str::<IpBan>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xbans\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xbans\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let ban = IpBan {
            id: from_row!(res->id()),
            ip: from_row!(res->ip()),
            reason: from_row!(res->reason()),
            moderator: match self
                .get_profile_by_id(res.get("moderator").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            timestamp: from_row!(res->timestamp(u128); 0),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.ipban:{}", id),
                serde_json::to_string::<IpBan>(&ban).unwrap(),
            )
            .await;

        // return
        Ok(ban)
    }

    /// Get an existing [`IpBan`] by its IP
    ///
    /// # Arguments
    /// * `ip`
    pub async fn get_ipban_by_ip(&self, ip: String) -> Result<IpBan> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xbans\" WHERE \"ip\" = ?"
        } else {
            "SELECT * FROM \"xbans\" WHERE \"ip\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&ip).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let ban = IpBan {
            id: from_row!(res->id()),
            ip: from_row!(res->ip()),
            reason: from_row!(res->reason()),
            moderator: match self
                .get_profile_by_id(res.get("moderator").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            timestamp: from_row!(res->timestamp(u128); 0),
        };

        // return
        Ok(ban)
    }

    /// Get all [`IpBan`]s
    ///
    /// # Arguments
    /// * `user` - the user doing this
    pub async fn get_ipbans(&self, user: Box<Profile>) -> Result<Vec<IpBan>> {
        // make sure user is a manager
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if !group.permissions.check(FinePermission::BAN_IP) {
            return Err(DatabaseError::NotAllowed);
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xbans\" ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xbans\" ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<IpBan> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(IpBan {
                        id: from_row!(res->id()),
                        ip: from_row!(res->ip()),
                        reason: from_row!(res->reason()),
                        moderator: match self
                            .get_profile_by_id(res.get("moderator").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                        timestamp: from_row!(res->timestamp(u128); 0),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    // SET
    /// Create a new [`IpBan`]
    ///
    /// # Arguments
    /// * `props` - [`IpBanCreate`]
    /// * `user` - the user creating this ban
    pub async fn create_ipban(&self, props: IpBanCreate, user: Box<Profile>) -> Result<()> {
        // make sure user is a helper
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if !group.permissions.check(FinePermission::BAN_IP) {
            return Err(DatabaseError::NotAllowed);
        } else {
            let actor_id = user.id.clone();
            if let Err(e) = self
                .create_notification(
                    NotificationCreate {
                        title: format!("[{actor_id}](/+u/{actor_id})"),
                        content: format!("Banned an IP: {}", props.ip),
                        address: format!("/+u/{actor_id}"),
                        recipient: "*(audit)".to_string(), // all staff, audit
                    },
                    None,
                )
                .await
            {
                return Err(e);
            }
        }

        // make sure this ip isn't already banned
        if self.get_ipban_by_ip(props.ip.clone()).await.is_ok() {
            return Err(DatabaseError::MustBeUnique);
        }

        // ...
        let ban = IpBan {
            id: utility::random_id(),
            ip: props.ip,
            reason: props.reason,
            moderator: user,
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create notification
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xbans\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xbans\" VALEUS ($1, $2, $3, $4, $5)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&ban.id)
            .bind::<&String>(&ban.ip)
            .bind::<&String>(&ban.reason)
            .bind::<&String>(&ban.moderator.id)
            .bind::<&String>(&ban.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => return Ok(()),
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing IpBan
    ///
    /// # Arguments
    /// * `id` - the ID of the ban
    /// * `user` - the user doing this
    pub async fn delete_ipban(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure ban exists
        let ipban = match self.get_ipban(id.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check id
        if user.id != ipban.moderator.id {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::UNBAN_IP) {
                return Err(DatabaseError::NotAllowed);
            } else {
                let actor_id = user.id.clone();
                if let Err(e) = self
                    .create_notification(
                        NotificationCreate {
                            title: format!("[{actor_id}](/+u/{actor_id})"),
                            content: format!("Unbanned an IP: {}", ipban.ip),
                            address: format!("/+u/{actor_id}"),
                            recipient: "*(audit)".to_string(), // all staff, audit
                        },
                        None,
                    )
                    .await
                {
                    return Err(e);
                }
            }
        }

        // delete ban
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xbans\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xbans\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.ipban:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // relationships

    /// Get the membership status of the given user and the other user
    ///
    /// # Arguments
    /// * `user` - the ID of the first user
    /// * `other` - the ID of the second user
    pub async fn get_user_relationship(
        &self,
        user: String,
        other: String,
    ) -> (RelationshipStatus, String, String) {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xrelationships\" WHERE (\"one\" = ? AND \"two\" = ?) OR (\"one\" = ? AND \"two\" = ?)"
        } else {
             "SELECT * FROM \"xrelationships\" WHERE (\"one\" = $1 AND \"two\" = $2) OR (\"one\" = $3 AND \"two\" = $4)"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&other)
            .bind::<&String>(&other)
            .bind::<&String>(&user)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return (RelationshipStatus::default(), user, other),
        };

        // return
        (
            serde_json::from_str(&res.get("status").unwrap()).unwrap(),
            res.get("one").unwrap().to_owned(),
            res.get("two").unwrap().to_owned(),
        )
    }

    /// Set the relationship of user `one` and `two`
    ///
    /// # Arguments
    /// * `one` - the ID of the first user
    /// * `two` - the ID of the second user
    /// * `status` - the new relationship status, setting to "Unknown" will remove the relationship
    /// * `disable_notifications`
    pub async fn set_user_relationship_status(
        &self,
        one: String,
        two: String,
        status: RelationshipStatus,
        disable_notifications: bool,
    ) -> Result<()> {
        // get current membership status
        let mut relationship = self.get_user_relationship(one.clone(), two.clone()).await;

        if relationship.0 == status {
            return Ok(());
        }

        let mut uone = match self.get_profile(relationship.1).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        let mut utwo = match self.get_profile(relationship.2).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // ...
        match status {
            RelationshipStatus::Blocked => {
                // if the relationship exists but we aren't user one, delete it
                if relationship.0 != RelationshipStatus::Unknown && uone.id != one {
                    // delete
                    let query: String =
                        if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                            "DELETE FROM \"xrelationships\" WHERE \"one\" = ? AND \"two\" = ?"
                        } else {
                            "DELETE FROM \"xrelationships\" WHERE \"one\" = ? AND \"two\" = ?"
                        }
                        .to_string();

                    let c = &self.base.db.client;
                    if let Err(_) = sqlquery(&query)
                        .bind::<&String>(&uone.id)
                        .bind::<&String>(&utwo.id)
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };

                    relationship.0 = RelationshipStatus::Unknown; // act like it never happened
                    uone.id = one;
                    utwo.id = two;
                }

                // ...
                if relationship.0 != RelationshipStatus::Unknown {
                    if relationship.0 == RelationshipStatus::Friends {
                        // decr friendship counts since we were previously friends but are not now
                        self.base
                            .cachedb
                            .decr(format!("rbeam.app.friends_count:{}", uone.id))
                            .await;

                        self.base
                            .cachedb
                            .decr(format!("rbeam.app.friends_count:{}", utwo.id))
                            .await;
                    }

                    // update
                    let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "UPDATE \"xrelationships\" SET \"status\" = ? WHERE \"one\" = ? AND \"two\" = ?"
                    } else {
                        "UPDATE \"xrelationships\" SET (\"status\") = (?) WHERE \"one\" = ? AND \"two\" = ?"
                    }
                    .to_string();

                    let c = &self.base.db.client;
                    if let Err(_) = sqlquery(&query)
                        .bind::<&String>(&serde_json::to_string(&status).unwrap())
                        .bind::<&String>(&uone.id)
                        .bind::<&String>(&utwo.id)
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                } else {
                    // add
                    let query: String =
                        if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                            "INSERT INTO \"xrelationships\" VALUES (?, ?, ?, ?)"
                        } else {
                            "INSERT INTO \"xrelationships\" VALEUS ($1, $2, $3, $4)"
                        }
                        .to_string();

                    let c = &self.base.db.client;
                    if let Err(_) = sqlquery(&query)
                        .bind::<&String>(&uone.id)
                        .bind::<&String>(&utwo.id)
                        .bind::<&String>(&serde_json::to_string(&status).unwrap())
                        .bind::<&String>(&rainbeam_shared::unix_epoch_timestamp().to_string())
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }
            }
            RelationshipStatus::Pending => {
                // check utwo permissions
                if utwo.metadata.is_true("sparkler:limited_friend_requests") {
                    // make sure utwo is following uone
                    if let Err(_) = self.get_follow(utwo.id.clone(), uone.id.clone()).await {
                        return Err(DatabaseError::NotAllowed);
                    }
                }

                // add
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "INSERT INTO \"xrelationships\" VALUES (?, ?, ?, ?)"
                    } else {
                        "INSERT INTO \"xrelationships\" VALEUS ($1, $2, $3, $4)"
                    }
                    .to_string();

                let c = &self.base.db.client;
                if let Err(_) = sqlquery(&query)
                    .bind::<&String>(&uone.id)
                    .bind::<&String>(&utwo.id)
                    .bind::<&String>(&serde_json::to_string(&status).unwrap())
                    .bind::<&String>(&rainbeam_shared::unix_epoch_timestamp().to_string())
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                // create notification
                if !disable_notifications {
                    if let Err(_) = self
                        .create_notification(
                            NotificationCreate {
                                title: format!(
                                    "[@{}](/+u/{}) has sent you a friend request!",
                                    uone.username, uone.id
                                ),
                                content: format!("{} wants to be your friend.", uone.username),
                                address: format!("/@{}/relationship/friend_accept", uone.id),
                                recipient: utwo.id,
                            },
                            None,
                        )
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                };
            }
            RelationshipStatus::Friends => {
                // update
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "UPDATE \"xrelationships\" SET \"status\" = ? WHERE \"one\" = ? AND \"two\" = ?"
                    } else {
                        "UPDATE \"xrelationships\" SET (\"status\") = (?) WHERE \"one\" = ? AND \"two\" = ?"
                    }
                    .to_string();

                let c = &self.base.db.client;
                if let Err(_) = sqlquery(&query)
                    .bind::<&String>(&serde_json::to_string(&status).unwrap())
                    .bind::<&String>(&uone.id)
                    .bind::<&String>(&utwo.id)
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                self.base
                    .cachedb
                    .incr(format!("rbeam.app.friends_count:{}", uone.id))
                    .await;

                self.base
                    .cachedb
                    .incr(format!("rbeam.app.friends_count:{}", utwo.id))
                    .await;

                // create notification
                if !disable_notifications {
                    if let Err(_) = self
                        .create_notification(
                            NotificationCreate {
                                title: "Your friend request has been accepted!".to_string(),
                                content: format!(
                                    "[@{}](/@{}) has accepted your friend request.",
                                    utwo.username, utwo.username
                                ),
                                address: String::new(),
                                recipient: uone.id,
                            },
                            None,
                        )
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                };
            }
            RelationshipStatus::Unknown => {
                // delete
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xrelationships\" WHERE \"one\" = ? AND \"two\" = ?"
                    } else {
                        "DELETE FROM \"xrelationships\" WHERE \"one\" = ? AND \"two\" = ?"
                    }
                    .to_string();

                let c = &self.base.db.client;
                if let Err(_) = sqlquery(&query)
                    .bind::<&String>(&uone.id)
                    .bind::<&String>(&utwo.id)
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                if relationship.0 == RelationshipStatus::Friends {
                    // decr friendship counts since we were previously friends but are not now
                    self.base
                        .cachedb
                        .decr(format!("rbeam.app.friends_count:{}", uone.id))
                        .await;

                    self.base
                        .cachedb
                        .decr(format!("rbeam.app.friends_count:{}", utwo.id))
                        .await;
                }
            }
        }

        // return
        Ok(())
    }

    /// Get all relationships owned by `user` (ownership is the relationship's `one` field)
    ///
    /// # Arguments
    /// * `user`
    pub async fn get_user_relationships(
        &self,
        user: String,
    ) -> Result<Vec<(Box<Profile>, RelationshipStatus)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xrelationships\" WHERE \"one\" = ?"
        } else {
            "SELECT * FROM \"xrelationships\" WHERE \"one\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    // get profile
                    let profile = match self.get_profile(res.get("two").unwrap().to_string()).await
                    {
                        Ok(c) => c,
                        Err(e) => return Err(e),
                    };

                    // add to out
                    out.push((
                        profile,
                        serde_json::from_str(&res.get("status").unwrap()).unwrap(),
                    ));
                }

                Ok(out)
            }
            Err(_) => return Err(DatabaseError::Other),
        }
    }

    /// Get all relationships owned by `user` (ownership is the relationship's `one` field)
    ///
    /// # Arguments
    /// * `user`
    /// * `status`
    pub async fn get_user_relationships_of_status(
        &self,
        user: String,
        status: RelationshipStatus,
    ) -> Result<Vec<(Box<Profile>, RelationshipStatus)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xrelationships\" WHERE \"one\" = ? AND \"status\" = ?"
        } else {
            "SELECT * FROM \"xrelationships\" WHERE \"one\" = $1 AND \"status\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&serde_json::to_string(&status).unwrap())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    // get profile
                    let profile = match self.get_profile(res.get("two").unwrap().to_string()).await
                    {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    // add to out
                    out.push((
                        profile,
                        serde_json::from_str(&res.get("status").unwrap()).unwrap(),
                    ));
                }

                Ok(out)
            }
            Err(_) => return Err(DatabaseError::Other),
        }
    }

    /// Get all relationships where `user` is either `one` or `two` and the status is `status`
    ///
    /// # Arguments
    /// * `user`
    /// * `status`
    pub async fn get_user_participating_relationships_of_status(
        &self,
        user: String,
        status: RelationshipStatus,
    ) -> Result<Vec<(Box<Profile>, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
           "SELECT * FROM \"xrelationships\" WHERE (\"one\" = ? OR \"two\" = ?) AND \"status\" = ?"
        } else {
           "SELECT * FROM \"xrelationships\" WHERE (\"one\" = $1 OR \"two\" = $2) AND \"status\" = $3"
        }.to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&user)
            .bind::<&String>(&serde_json::to_string(&status).unwrap())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    // get profiles
                    let profile = match self.get_profile(res.get("one").unwrap().to_string()).await
                    {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let profile_2 =
                        match self.get_profile(res.get("two").unwrap().to_string()).await {
                            Ok(c) => c,
                            Err(_) => continue,
                        };

                    // add to out
                    out.push((profile, profile_2));
                }

                Ok(out)
            }
            Err(_) => return Err(DatabaseError::Other),
        }
    }

    /// Get all relationships where `user` is either `one` or `two` and the status is `status`, 12 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `status`
    /// * `page`
    pub async fn get_user_participating_relationships_of_status_paginated(
        &self,
        user: String,
        status: RelationshipStatus,
        page: i32,
    ) -> Result<Vec<(Box<Profile>, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xrelationships\" WHERE (\"one\" = ? OR \"two\" = ?) AND \"status\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xrelationships\" WHERE (\"one\" = $1 OR \"two\" = $2) AND \"status\" = $3 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&user)
            .bind::<&String>(&serde_json::to_string(&status).unwrap())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    // get profiles
                    let profile = match self.get_profile(res.get("one").unwrap().to_string()).await
                    {
                        Ok(c) => c,
                        Err(e) => return Err(e),
                    };

                    let profile_2 =
                        match self.get_profile(res.get("two").unwrap().to_string()).await {
                            Ok(c) => c,
                            Err(e) => return Err(e),
                        };

                    // add to out
                    out.push((profile, profile_2));
                }

                Ok(out)
            }
            Err(_) => return Err(DatabaseError::Other),
        }
    }

    /// Get the number of friends a user has
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_friendship_count_by_user(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("rbeam.app.friends_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_user_participating_relationships_of_status(id.clone(), RelationshipStatus::Friends)
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(format!("rbeam.app.friends_count:{}", id), count.to_string())
            .await;

        count
    }

    // ip blocks

    // GET
    /// Get an existing [`IpBlock`]
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_ipblock(&self, id: String) -> Result<IpBlock> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.ipblock:{}", id))
            .await
        {
            Some(c) => return Ok(serde_json::from_str::<IpBlock>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xipblocks\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xipblocks\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let block = IpBlock {
            id: from_row!(res->id()),
            ip: from_row!(res->ip()),
            user: res.get("user").unwrap().to_string(),
            context: res.get("context").unwrap().to_string(),
            timestamp: from_row!(res->timestamp(u128); 0),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.ipblock:{}", id),
                serde_json::to_string::<IpBlock>(&block).unwrap(),
            )
            .await;

        // return
        Ok(block)
    }

    /// Get an existing [`IpBlock`] by its IP and its `user`
    ///
    /// # Arguments
    /// * `ip`
    /// * `user`
    pub async fn get_ipblock_by_ip(&self, ip: String, user: String) -> Result<IpBlock> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xipblocks\" WHERE \"ip\" = ? AND \"user\" = ?"
        } else {
            "SELECT * FROM \"xipblocks\" WHERE \"ip\" = $1 AND \"user\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&ip)
            .bind::<&String>(&user)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let block = IpBlock {
            id: from_row!(res->id()),
            ip: from_row!(res->ip()),
            user: res.get("user").unwrap().to_string(),
            context: res.get("context").unwrap().to_string(),
            timestamp: from_row!(res->timestamp(u128); 0),
        };

        // return
        Ok(block)
    }

    /// Get all [`IpBlocks`]s for the given `query_user`
    ///
    /// # Arguments
    /// * `query_user` - the ID of the user the blocks belong to
    pub async fn get_ipblocks(&self, query_user: String) -> Result<Vec<IpBlock>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xipblocks\" WHERE \"user\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xipblocks\" WHERE \"user\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&query_user)
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<IpBlock> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    out.push(IpBlock {
                        id: from_row!(res->id()),
                        ip: from_row!(res->ip()),
                        user: res.get("user").unwrap().to_string(),
                        context: res.get("context").unwrap().to_string(),
                        timestamp: from_row!(res->timestamp(u128); 0),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    // SET
    /// Create a new [`IpBlock`]
    ///
    /// # Arguments
    /// * `props` - [`IpBlockCreate`]
    /// * `user` - the user creating this block
    pub async fn create_ipblock(&self, props: IpBlockCreate, user: Box<Profile>) -> Result<()> {
        // make sure this ip isn't already banned
        if self
            .get_ipblock_by_ip(props.ip.clone(), user.id.clone())
            .await
            .is_ok()
        {
            return Err(DatabaseError::MustBeUnique);
        }

        // ...
        let block = IpBlock {
            id: utility::random_id(),
            ip: props.ip,
            user: user.id,
            context: props.context,
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create notification
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xipblocks\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xipblocks\" VALEUS ($1, $2, $3, $4, $5)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&block.id)
            .bind::<&String>(&block.ip)
            .bind::<&String>(&block.user)
            .bind::<&String>(&block.context)
            .bind::<&String>(&block.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => return Ok(()),
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing IpBlock
    ///
    /// # Arguments
    /// * `id` - the ID of the block
    /// * `user` - the user doing this
    pub async fn delete_ipblock(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure block exists
        let block = match self.get_ipblock(id.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check id
        if user.id != block.user {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::UNBAN_IP) {
                return Err(DatabaseError::NotAllowed);
            } else {
                let actor_id = user.id.clone();
                if let Err(e) = self
                    .create_notification(
                        NotificationCreate {
                            title: format!("[{actor_id}](/+u/{actor_id})"),
                            content: format!("Unblocked an IP: {}", block.ip),
                            address: format!("/+u/{actor_id}"),
                            recipient: "*(audit)".to_string(), // all staff, audit
                        },
                        None,
                    )
                    .await
                {
                    return Err(e);
                }
            }
        }

        // delete block
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xipblocks\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xipblocks\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.ipblock:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // mail

    // GET
    /// Get an existing mail
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_mail(&self, id: String) -> Result<Mail> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.mail:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<Mail>(c.as_str()) {
                Ok(c) => return Ok(c),
                Err(_) => {
                    self.base
                        .cachedb
                        .remove(format!("rbeam.auth.mail:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xmail\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xmail\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let recipient = res.get("recipient").unwrap();
        let mail = Mail {
            title: res.get("title").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            timestamp: from_row!(res->timestamp(u128); 0),
            id: from_row!(res->id()),
            state: serde_json::from_str(res.get("state").unwrap()).unwrap(),
            author: res.get("author").unwrap().to_string(),
            recipient: serde_json::from_str(recipient).unwrap_or(vec![recipient.to_string()]),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.mail:{}", id),
                serde_json::to_string::<Mail>(&mail).unwrap(),
            )
            .await;

        // return
        Ok(mail)
    }

    /// Get all mail by their recipient, 12 at a time
    ///
    /// # Arguments
    /// * `recipient`
    /// * `page`
    pub async fn get_mail_by_recipient_paginated(
        &self,
        recipient: String,
        page: i32,
    ) -> Result<Vec<(Mail, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xmail\" WHERE \"recipient\" LIKE ? OR \"recipient\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xmail\" WHERE \"recipient\" LIKE $1 OR \"recipient\" = $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%\"{}\"%", recipient.to_lowercase()))
            .bind::<&String>(&recipient.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Mail, Box<Profile>)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let author = res.get("author").unwrap();
                    let recipient = res.get("recipient").unwrap();

                    out.push((
                        Mail {
                            title: res.get("title").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            id: from_row!(res->id()),
                            state: serde_json::from_str(res.get("state").unwrap()).unwrap(),
                            author: author.to_string(),
                            recipient: serde_json::from_str(recipient)
                                .unwrap_or(vec![recipient.to_string()]),
                        },
                        match self.get_profile(author.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all mail by their recipient, 12 at a time
    ///
    /// # Arguments
    /// * `recipient`
    /// * `page`
    pub async fn get_mail_by_recipient_sent_paginated(
        &self,
        recipient: String,
        page: i32,
    ) -> Result<Vec<(Mail, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xmail\" WHERE \"author\" LIKE ? OR \"author\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xmail\" WHERE \"author\" LIKE $1 OR \"author\" = $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%\"{}\"%", recipient.to_lowercase()))
            .bind::<&String>(&recipient.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Mail, Box<Profile>)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;

                    let author = res.get("author").unwrap();
                    let recipient = res.get("recipient").unwrap();

                    out.push((
                        Mail {
                            title: res.get("title").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            timestamp: from_row!(res->timestamp(u128); 0),
                            id: from_row!(res->id()),
                            state: serde_json::from_str(res.get("state").unwrap()).unwrap(),
                            author: author.to_string(),
                            recipient: serde_json::from_str(recipient)
                                .unwrap_or(vec![recipient.to_string()]),
                        },
                        match self.get_profile(author.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    // SET
    /// Create a new mail
    ///
    /// # Arguments
    /// * `props` - [`MailCreate`]
    pub async fn create_mail(&self, props: MailCreate, author: String) -> Result<Mail> {
        // check content length
        if props.title.len() > (64 * 4) {
            return Err(DatabaseError::Other);
        }

        if props.title.len() < 2 {
            return Err(DatabaseError::Other);
        }

        if props.content.len() > (64 * 8) {
            return Err(DatabaseError::Other);
        }

        if props.content.len() < 2 {
            return Err(DatabaseError::Other);
        }

        // check markdown content
        let markdown = rainbeam_shared::ui::render_markdown(&props.content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::Other);
        }

        // make sure author and recipient exist
        let author = match self.get_profile(author.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        let mut recipients = Vec::new();

        for recipient in props.recipient {
            if recipient.is_empty() {
                continue;
            }

            match self.get_profile(recipient.clone()).await {
                Ok(ua) => {
                    if ua.metadata.is_true("sparkler:disable_mailbox") {
                        // continue so the mail is not delivered to this person
                        continue;
                    }

                    // check relationship
                    let relationship = self
                        .get_user_relationship(ua.id.clone(), author.id.clone())
                        .await
                        .0;

                    if relationship == RelationshipStatus::Blocked {
                        // continue so the mail is not delivered to this person
                        continue;
                    }

                    // ...
                    recipients.push(ua.id)
                }
                Err(e) => return Err(e),
            }
        }

        if recipients.len() == 0 {
            // can't send to nobody!
            return Err(DatabaseError::ValueError);
        };

        // ...
        let mail = Mail {
            title: props.title,
            content: props.content,
            timestamp: utility::unix_epoch_timestamp(),
            // id: utility::random_id(),
            id: AlmostSnowflake::new(self.config.snowflake_server_id).to_string(),
            state: MailState::Unread,
            author: author.id.clone(),
            recipient: recipients.clone(),
        };

        // create mail
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xmail\" VALUES (?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xmail\" VALEUS ($1, $2, $3, $4, $5, $6, $7)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&mail.title)
            .bind::<&String>(&mail.content)
            .bind::<&String>(&mail.timestamp.to_string())
            .bind::<&String>(&mail.id)
            .bind::<&String>(&serde_json::to_string(&mail.state).unwrap())
            .bind::<&String>(&mail.author)
            .bind::<&String>(&serde_json::to_string(&mail.recipient).unwrap())
            .execute(c)
            .await
        {
            Ok(_) => {
                // send notification
                for recipient in recipients {
                    if let Err(e) = self
                        .create_notification(
                            NotificationCreate {
                                title: format!(
                                    "[@{}](/+u/{}) sent you new mail!",
                                    author.username, author.id
                                ),
                                content: String::new(),
                                address: format!("/inbox/mail/letter/{}", mail.id),
                                recipient: recipient.clone(),
                            },
                            None,
                        )
                        .await
                    {
                        return Err(e);
                    };
                }

                // ...
                return Ok(mail);
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing mail
    ///
    /// Mail can only be deleted by their recipient.
    ///
    /// # Arguments
    /// * `id` - the ID of the mail
    /// * `user` - the user doing this
    pub async fn delete_mail(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure mail exists
        let mail = match self.get_mail(id.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check username
        if !mail.recipient.contains(&user.id) && (user.id != mail.author) {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_MAILS) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete mail
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xmail\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xmail\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.mail:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Update mail state
    ///
    /// # Arguments
    /// * `id`
    /// * `state` - [`MailState`]
    /// * `user`
    pub async fn update_mail_state(
        &self,
        id: String,
        state: MailState,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure mail exists and check permission
        let mail = match self.get_mail(id.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != mail.author {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::MANAGE_MAILS) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // update mail
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xmail\" SET \"state\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xmail\" SET (\"state\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&serde_json::to_string(&state).unwrap())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.mail:{}", id))
                    .await;

                // ...
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // labels

    /// Get a [`UserLabel`] from a database result
    pub async fn gimme_label(&self, res: BTreeMap<String, String>) -> Result<UserLabel> {
        Ok(UserLabel {
            id: from_row!(res->id()),
            name: res.get("name").unwrap().to_string(),
            timestamp: from_row!(res->timestamp(u128); 0),
            creator: res.get("author").unwrap().to_string(),
        })
    }

    /// Get an existing label
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_label(&self, id: String) -> Result<UserLabel> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.label:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<UserLabel>(c.as_str()) {
                Ok(c) => return Ok(c),
                Err(_) => {
                    self.base
                        .cachedb
                        .remove(format!("rbeam.auth.label:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xlabels\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xlabels\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let label = match self.gimme_label(res).await {
            Ok(l) => l,
            Err(e) => return Err(e),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.label:{}", id),
                serde_json::to_string::<UserLabel>(&label).unwrap(),
            )
            .await;

        // return
        Ok(label)
    }

    /// Create a new user label
    ///
    /// # Arguments
    /// * `name` - the name of the label
    /// * `author` - the ID of the user creating the label
    pub async fn create_label(&self, name: String, author: String) -> Result<UserLabel> {
        // check author permissions
        let author = match self.get_profile(author.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        let group = match self.get_group_by_id(author.group).await {
            Ok(g) => g,
            Err(e) => return Err(e),
        };

        if !group.permissions.check(FinePermission::CREATE_LABEL) {
            return Err(DatabaseError::NotAllowed);
        }

        // check name length
        if name.len() < 2 {
            return Err(DatabaseError::Other);
        }

        if name.len() > 32 {
            return Err(DatabaseError::Other);
        }

        // ...
        let label = UserLabel {
            // id: utility::random_id(),
            id: AlmostSnowflake::new(self.config.snowflake_server_id).to_string(),
            name,
            timestamp: utility::unix_epoch_timestamp(),
            creator: author.id,
        };

        // create page
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xlabels\" VALUES (?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xlabels\" VALEUS ($1, $2, $3, $4)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&label.id)
            .bind::<&String>(&label.name)
            .bind::<&String>(&label.timestamp.to_string())
            .bind::<&String>(&label.creator)
            .execute(c)
            .await
        {
            Ok(_) => Ok(label),
            Err(_) => Err(DatabaseError::Other),
        }
    }

    // ugc transactions

    // Get item given the `row` data
    pub async fn gimme_transaction(
        &self,
        row: BTreeMap<String, String>,
    ) -> Result<(Transaction, Option<Item>)> {
        let item = row.get("item").unwrap().to_string();

        Ok((
            Transaction {
                id: row.get("id").unwrap().to_string(),
                amount: row.get("amount").unwrap().parse::<i32>().unwrap_or(0),
                item: item.clone(),
                timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
                customer: row.get("customer").unwrap().to_string(),
                merchant: row.get("merchant").unwrap().to_string(),
            },
            match self.get_item(item).await {
                Ok(i) => Some(i),
                Err(_) => None,
            },
        ))
    }

    // GET
    /// Get an existing transaction
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_transaction(&self, id: String) -> Result<(Transaction, Option<Item>)> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.econ.transaction:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<BTreeMap<String, String>>(c.as_str()) {
                Ok(c) => {
                    return Ok(match self.gimme_transaction(c).await {
                        Ok(t) => t,
                        Err(e) => return Err(e),
                    })
                }
                Err(_) => {
                    self.base
                        .cachedb
                        .remove(format!("rbeam.auth.econ.transaction:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xugc_transactions\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xugc_transactions\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let transaction = match self.gimme_transaction(res).await {
            Ok(t) => t,
            Err(e) => return Err(e),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.econ.transaction:{}", id),
                serde_json::to_string::<Transaction>(&transaction.0).unwrap(),
            )
            .await;

        // return
        Ok(transaction)
    }

    /// Get an existing transaction given the `customer` and the `item`
    ///
    /// # Arguments
    /// * `user`
    /// * `item`
    pub async fn get_transaction_by_customer_item(
        &self,
        customer: String,
        item: String,
    ) -> Result<(Transaction, Option<Item>)> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xugc_transactions\" WHERE \"customer\" = ? AND \"item\" = ?"
        } else {
            "SELECT * FROM \"xugc_transactions\" WHERE \"customer\" = $1 AND \"item\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&customer)
            .bind::<&String>(&item)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let transaction = match self.gimme_transaction(res).await {
            Ok(t) => t,
            Err(e) => return Err(e),
        };

        // return
        Ok(transaction)
    }

    /// Get all transactions where the customer is the given user ID, 12 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `page`
    ///
    /// # Returns
    /// `Vec<(Transaction, Customer, Merchant)>`
    pub async fn get_transactions_by_customer_paginated(
        &self,
        user: String,
        page: i32,
    ) -> Result<Vec<((Transaction, Option<Item>), Box<Profile>, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xugc_transactions\" WHERE \"customer\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xugc_transactions\" WHERE \"customer\" = $1 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&user)
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let transaction = match self.gimme_transaction(res).await {
                        Ok(t) => t,
                        Err(e) => return Err(e),
                    };

                    let customer = transaction.0.customer.clone();
                    let merchant = transaction.0.merchant.clone();

                    out.push((
                        transaction,
                        match self.get_profile(customer.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                        match self.get_profile(merchant.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all transactions by the given user ID, 12 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `page`
    ///
    /// # Returns
    /// `Vec<(Transaction, Customer, Merchant)>`
    pub async fn get_participating_transactions_paginated(
        &self,
        user: String,
        page: i32,
    ) -> Result<Vec<((Transaction, Option<Item>), Box<Profile>, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xugc_transactions\" WHERE \"customer\" = ? OR \"merchant\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xugc_transactions\" WHERE \"customer\" = $1 OR \"merchant\" = $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&user)
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let transaction = match self.gimme_transaction(res).await {
                        Ok(t) => t,
                        Err(e) => return Err(e),
                    };

                    let customer = transaction.0.customer.clone();
                    let merchant = transaction.0.merchant.clone();

                    out.push((
                        transaction,
                        match self.get_profile(customer.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                        match self.get_profile(merchant.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    // SET
    /// Create a new transaction
    ///
    /// # Arguments
    /// * `props` - [`TransactionCreate`]
    /// * `customer` - the user in the `customer` field of the transaction
    pub async fn create_transaction(
        &self,
        props: TransactionCreate,
        customer: String,
    ) -> Result<Transaction> {
        // make sure customer and merchant exist
        let customer = match self.get_profile(customer.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        let merchant = match self.get_profile(props.merchant.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // make sure customer can afford this
        if props.amount.is_negative() {
            if (customer.coins + props.amount) < 0 {
                return Err(DatabaseError::TooExpensive);
            }
        }

        // ...
        let transaction = Transaction {
            // id: utility::random_id(),
            id: AlmostSnowflake::new(self.config.snowflake_server_id).to_string(),
            amount: props.amount,
            item: props.item,
            timestamp: utility::unix_epoch_timestamp(),
            customer: customer.id.clone(),
            merchant: merchant.id.clone(),
        };

        // create transaction
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xugc_transactions\" VALUES (?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xugc_transactions\" VALEUS ($1, $2, $3, $4, $5, $6)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&transaction.id)
            .bind::<&String>(&transaction.amount.to_string())
            .bind::<&String>(&transaction.item)
            .bind::<&String>(&transaction.timestamp.to_string())
            .bind::<&String>(&transaction.customer)
            .bind::<&String>(&transaction.merchant)
            .execute(c)
            .await
        {
            Ok(_) => {
                // update balances
                if let Err(e) = self
                    .update_profile_coins(customer.id.clone(), transaction.amount)
                    .await
                {
                    return Err(e);
                };

                if let Err(e) = self
                    .update_profile_coins(merchant.id.clone(), transaction.amount.abs())
                    .await
                {
                    return Err(e);
                };

                // send notification
                if (customer.id != merchant.id) && (merchant.id != "0".to_string()) {
                    if let Err(e) = self
                        .create_notification(
                            NotificationCreate {
                                title: "Purchased data now available!".to_string(),
                                content: "Data from an item you purchased is now available."
                                    .to_string(),
                                address: format!(
                                    "/market/item/{}#/preview",
                                    transaction.item.clone()
                                ),
                                recipient: customer.id,
                            },
                            None,
                        )
                        .await
                    {
                        return Err(e);
                    }
                }

                // ...
                return Ok(transaction);
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // ugc items

    // Get transaction given the `row` data
    pub fn gimme_item(&self, row: BTreeMap<String, String>) -> Result<Item> {
        Ok(Item {
            id: row.get("id").unwrap().to_string(),
            name: row.get("name").unwrap().to_string(),
            description: row.get("description").unwrap().to_string(),
            cost: row.get("cost").unwrap().parse::<i32>().unwrap_or(0),
            content: row.get("content").unwrap().to_string(),
            r#type: match serde_json::from_str(row.get("type").unwrap()) {
                Ok(v) => v,
                Err(_) => return Err(DatabaseError::ValueError),
            },
            status: match serde_json::from_str(row.get("status").unwrap()) {
                Ok(v) => v,
                Err(_) => return Err(DatabaseError::ValueError),
            },
            timestamp: row.get("timestamp").unwrap().parse::<u128>().unwrap(),
            creator: row.get("creator").unwrap().to_string(),
        })
    }

    // GET
    /// Get an existing item
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_item(&self, id: String) -> Result<Item> {
        if id == "0" {
            // this item can be charged for things that don't relate to an item
            return Ok(Item {
                id: "0".to_string(),
                name: "System cost".to_string(),
                description: String::new(),
                cost: -1,
                content: String::new(),
                r#type: ItemType::Text,
                status: ItemStatus::Approved,
                timestamp: 0,
                creator: "0".to_string(),
            });
        }

        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.auth.econ.item:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<Item>(c.as_str()) {
                Ok(c) => return Ok(c),
                Err(_) => {
                    self.base
                        .cachedb
                        .remove(format!("rbeam.auth.econ.item:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xugc_items\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xugc_items\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let item = match self.gimme_item(res) {
            Ok(t) => t,
            Err(e) => return Err(e),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.auth.econ.item:{}", id),
                serde_json::to_string::<Item>(&item).unwrap(),
            )
            .await;

        // return
        Ok(item)
    }

    /// Get all items by their creator, 12 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `page`
    ///
    /// # Returns
    /// `Vec<(Item, Box<Profile>)>`
    pub async fn get_items_by_creator_paginated(
        &self,
        user: String,
        page: i32,
    ) -> Result<Vec<(Item, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xugc_items\" WHERE \"creator\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xugc_items\" WHERE \"creator\" = $1 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let item = match self.gimme_item(res) {
                        Ok(t) => t,
                        Err(e) => return Err(e),
                    };

                    let creator = item.creator.clone();

                    out.push((
                        item,
                        match self.get_profile(creator.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all items by their creator and type, 12 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `type`
    /// * `page`
    ///
    /// # Returns
    /// `Vec<(Item, Box<Profile>)>`
    pub async fn get_items_by_creator_type_paginated(
        &self,
        user: String,
        r#type: ItemType,
        page: i32,
    ) -> Result<Vec<(Item, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xugc_items\" WHERE \"creator\" = ? AND \"type\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xugc_items\" WHERE \"creator\" = $1 AND \"type\" = $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&serde_json::to_string(&r#type).unwrap())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let item = match self.gimme_item(res) {
                        Ok(t) => t,
                        Err(e) => return Err(e),
                    };

                    let creator = item.creator.clone();

                    out.push((
                        item,
                        match self.get_profile(creator.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all items by their status, 12 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `page`
    ///
    /// # Returns
    /// `Vec<(Item, Box<Profile>)>`
    pub async fn get_items_by_status_searched_paginated(
        &self,
        status: ItemStatus,
        page: i32,
        search: String,
    ) -> Result<Vec<(Item, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xugc_items\" WHERE \"status\" = ? AND \"name\" LIKE ? AND \"cost\" != '-1' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xugc_items\" WHERE \"status\" = $1 AND \"name\" LIKE $2 AND \"cost\" != '-1' ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&serde_json::to_string(&status).unwrap())
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let item = match self.gimme_item(res) {
                        Ok(t) => t,
                        Err(e) => return Err(e),
                    };

                    let creator = item.creator.clone();

                    out.push((
                        item,
                        match self.get_profile(creator.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all items by their type, 12 at a time
    ///
    /// # Arguments
    /// * `type`
    /// * `page`
    ///
    /// # Returns
    /// `Vec<(Item, Box<Profile>)>`
    pub async fn get_items_by_type_paginated(
        &self,
        r#type: ItemType,
        page: i32,
    ) -> Result<Vec<(Item, Box<Profile>)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xugc_items\" WHERE \"type\" = ? ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        } else {
            format!("SELECT * FROM \"xugc_items\" WHERE \"type\" = $2 ORDER BY \"timestamp\" DESC LIMIT 12 OFFSET {}", page * 12)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&serde_json::to_string(&r#type).unwrap())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row).0;
                    let item = match self.gimme_item(res) {
                        Ok(t) => t,
                        Err(e) => return Err(e),
                    };

                    let creator = item.creator.clone();

                    out.push((
                        item,
                        match self.get_profile(creator.to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    // SET
    /// Create a new item
    ///
    /// # Arguments
    /// * `props` - [`ItemCreate`]
    /// * `creator` - the user in the `creator` field of the item
    pub async fn create_item(&self, props: ItemCreate, creator: String) -> Result<Item> {
        // check values
        if props.content.len() > (64 * 128 * 2) {
            return Err(DatabaseError::TooLong);
        }

        if props.content.len() < 2 {
            return Err(DatabaseError::ValueError);
        }

        if props.name.len() > (64 * 2) {
            return Err(DatabaseError::TooLong);
        }

        if props.name.len() < 2 {
            return Err(DatabaseError::ValueError);
        }

        if props.description.len() > (64 * 128) {
            return Err(DatabaseError::TooLong);
        }

        // if we're creating a module, the cost HAS to be -1 (offsale)
        if props.r#type == ItemType::Module && props.cost != -1 {
            return Err(DatabaseError::NotAllowed);
        }

        // ...
        if props.cost.is_negative() && props.cost != -1 {
            return Err(DatabaseError::NotAllowed);
        }

        let item = Item {
            // id: utility::random_id(),
            id: AlmostSnowflake::new(self.config.snowflake_server_id).to_string(),
            name: props.name,
            description: props.description,
            cost: props.cost,
            content: props.content,
            r#type: props.r#type,
            status: ItemStatus::Pending,
            timestamp: utility::unix_epoch_timestamp(),
            creator: creator.clone(),
        };

        // subtract creation cost from creator
        /* if let Err(e) = self
            .create_transaction(
                TransactionCreate {
                    merchant: "0".to_string(),
                    item: "0".to_string(),
                    amount: -25,
                },
                creator.clone(),
            )
            .await
        {
            return Err(e);
        }; */

        // create item
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xugc_items\" VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xugc_items\" VALEUS ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&item.id)
            .bind::<&String>(&item.name)
            .bind::<&String>(&item.description)
            .bind::<&String>(&item.cost.to_string())
            .bind::<&String>(&item.content)
            .bind::<&String>(&serde_json::to_string(&item.r#type).unwrap())
            .bind::<&String>(&serde_json::to_string(&item.status).unwrap())
            .bind::<&String>(&item.timestamp.to_string())
            .bind::<&String>(&item.creator)
            .execute(c)
            .await
        {
            Ok(_) => {
                // buy item (for free)
                if let Err(e) = self
                    .create_transaction(
                        TransactionCreate {
                            merchant: creator.clone(),
                            item: item.id.clone(),
                            amount: 0,
                        },
                        creator.clone(),
                    )
                    .await
                {
                    return Err(e);
                };

                // ...
                return Ok(item);
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Update item status
    ///
    /// # Arguments
    /// * `id`
    /// * `status` - [`ItemStatus`]
    /// * `user`
    pub async fn update_item_status(
        &self,
        id: String,
        status: ItemStatus,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure item exists and check permission
        let item = match self.get_item(id.clone()).await {
            Ok(i) => i,
            Err(e) => return Err(e),
        };

        // check permission
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if !group.permissions.check(FinePermission::ECON_MASTER) {
            return Err(DatabaseError::NotAllowed);
        }

        // update item
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xugc_items\" SET \"status\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xugc_items\" SET (\"status\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&serde_json::to_string(&status).unwrap())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                if let Err(e) = self
                    .create_notification(
                        NotificationCreate {
                            title: "Item status updated!".to_string(),
                            content: format!(
                                "An item you created has been updated to the status of \"{}\"",
                                status.to_string()
                            ),
                            address: format!("/market/item/{}", item.id.clone()),
                            recipient: item.creator,
                        },
                        None,
                    )
                    .await
                {
                    return Err(e);
                }

                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.econ.item:{}", id))
                    .await;

                // ...
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Update item fields
    ///
    /// # Arguments
    /// * `id`
    /// * `props` - [`ItemEdit`]
    /// * `user`
    pub async fn update_item(&self, id: String, props: ItemEdit, user: Box<Profile>) -> Result<()> {
        // make sure item exists and check permission
        let item = match self.get_item(id.clone()).await {
            Ok(i) => i,
            Err(e) => return Err(e),
        };

        // check values
        if props.name.len() > (64 * 2) {
            return Err(DatabaseError::TooLong);
        }

        if props.name.len() < 2 {
            return Err(DatabaseError::ValueError);
        }

        if props.description.len() > (64 * 128) {
            return Err(DatabaseError::TooLong);
        }

        // if we're creating a module, the cost HAS to be -1 (offsale)
        if item.r#type == ItemType::Module && props.cost != -1 {
            return Err(DatabaseError::NotAllowed);
        }

        // ...
        if props.cost.is_negative() && props.cost != -1 {
            return Err(DatabaseError::NotAllowed);
        }

        // check permission
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if user.id != item.creator {
            if !group.permissions.check(FinePermission::ECON_MASTER) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // update item
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xugc_items\" SET \"name\" = ?, \"description\" = ?, \"cost\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xugc_items\" SET (\"name\", \"description\", \"cost\") = ($1, $2, $3) WHERE \"id\" = $4"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&props.name)
            .bind::<&String>(&props.description)
            .bind::<i32>(props.cost)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.econ.item:{}", id))
                    .await;

                // ...
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Update item content
    ///
    /// # Arguments
    /// * `id`
    /// * `props` - [`ItemEditContent`]
    /// * `user`
    pub async fn update_item_content(
        &self,
        id: String,
        props: ItemEditContent,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure item exists and check permission
        let item = match self.get_item(id.clone()).await {
            Ok(i) => i,
            Err(e) => return Err(e),
        };

        // we cannot change the content of a module
        // this would allow people to lie about the checksum of their wasm package
        //
        // doing this also ensures that people create a new asset for each version of their package,
        // meaning old versions will still verify properly
        if item.r#type == ItemType::Module {
            return Err(DatabaseError::NotAllowed);
        }

        // check values
        if props.content.len() > (64 * 128 * 2) {
            return Err(DatabaseError::TooLong);
        }

        if props.content.len() < 2 {
            return Err(DatabaseError::ValueError);
        }

        // check permission
        let group = match self.get_group_by_id(user.group).await {
            Ok(g) => g,
            Err(_) => return Err(DatabaseError::Other),
        };

        if user.id != item.creator {
            if !group.permissions.check(FinePermission::ECON_MASTER) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // update item
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xugc_items\" SET \"content\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xugc_items\" SET (\"content\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&props.content)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.econ.item:{}", id))
                    .await;

                // ...
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing item
    ///
    /// Items can only be deleted by their creator.
    ///
    /// # Arguments
    /// * `id` - the ID of the item
    /// * `user` - the user doing this
    pub async fn delete_item(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure item exists
        let item = match self.get_item(id.clone()).await {
            Ok(n) => n,
            Err(e) => return Err(e),
        };

        // check username
        if item.creator != user.id {
            // check permission
            let group = match self.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.check(FinePermission::ECON_MASTER) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete mail
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xugc_items\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xugc_items\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.econ.item:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // totp

    /// Update the profile's TOTP secret.
    pub async fn update_profile_totp_secret(
        &self,
        id: String,
        secret: String,
        recovery: &Vec<String>,
    ) -> Result<()> {
        let ua = match self.get_profile_by_id(id.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // update profile
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xprofiles\" SET \"totp\" = ?, \"recovery_codes\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xprofiles\" SET (\"totp\", \"recovery_codes\") = ($1, $2) WHERE \"id\" = $3"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&secret)
            .bind::<&String>(&serde_json::to_string(&recovery).unwrap())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{id}"))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.auth.profile:{}", ua.username))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Enable TOTP for a profile.
    ///
    /// # Returns
    /// `Result<(secret, qr base64)>`
    pub async fn enable_totp(
        &self,
        as_user: Box<Profile>,
        id: String,
    ) -> Result<(String, String, Vec<String>)> {
        let profile = match self.get_profile(id.clone()).await {
            Ok(p) => p,
            Err(e) => return Err(e),
        };

        if profile.id != as_user.id {
            return Err(DatabaseError::NotAllowed);
        }

        let secret = totp_rs::Secret::default();
        let recovery = Database::generate_totp_recovery_codes();

        // update profile
        if let Err(e) = self
            .update_profile_totp_secret(id.to_string(), secret.to_string(), &recovery)
            .await
        {
            return Err(e);
        }

        // fetch profile again (with totp information)
        let profile = match self.get_profile(id.clone()).await {
            Ok(p) => p,
            Err(e) => return Err(e),
        };

        // get totp
        let totp = profile.totp(Some(
            self.config
                .host
                .replace("http://", "")
                .replace("https://", "")
                .replace(":", "_"),
        ));

        if totp.is_none() {
            return Err(DatabaseError::Other);
        }

        let totp = totp.unwrap();

        // generate qr
        let qr = match totp.get_qr_base64() {
            Ok(q) => q,
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok((secret.to_string(), qr, recovery))
    }

    /// Validate a given TOTP code for the given profile.
    pub fn check_totp(&self, ua: &Box<Profile>, code: &str) -> bool {
        let totp = ua.totp(Some(
            self.config
                .host
                .replace("http://", "")
                .replace("https://", "")
                .replace(":", "_"),
        ));

        if let Some(totp) = totp {
            return !code.is_empty()
                && (totp.check_current(code).unwrap()
                    | ua.recovery_codes.contains(&code.to_string()));
        }

        true
    }

    /// Generate 8 random recovery codes for TOTP.
    pub fn generate_totp_recovery_codes() -> Vec<String> {
        let mut out: Vec<String> = Vec::new();

        for _ in 0..9 {
            out.push(rainbeam_shared::hash::salt())
        }

        out
    }
}
