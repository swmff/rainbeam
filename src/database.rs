use crate::config::Config;
use crate::model::{
    anonymous_profile, global_profile, CommentCreate, QuestionCreate, QuestionResponse,
    RefQuestion, ResponseComment, ResponseCreate,
};
use crate::model::{DatabaseError, Question};

use xsu_authman::model::{NotificationCreate, Permission, Profile};
use xsu_dataman::query as sqlquery;
use xsu_dataman::utility;

pub type Result<T> = std::result::Result<T, DatabaseError>;

/// Database connector
#[derive(Clone)]
pub struct Database {
    pub base: xsu_dataman::StarterDatabase,
    pub auth: xsu_authman::Database,
    pub server_options: Config,
}

impl Database {
    pub async fn new(
        opts: xsu_dataman::DatabaseOpts,
        auth: xsu_authman::Database,
        server_options: Config,
    ) -> Self {
        Self {
            base: xsu_dataman::StarterDatabase::new(opts).await,
            auth,
            server_options,
        }
    }

    /// Init database
    pub async fn init(&self) {
        // create tables
        let c = &self.base.db.client;

        // create questions table
        // we're only going to store unanswered questions here
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xquestions\" (
                author    TEXT,
                recipient TEXT,
                content   TEXT,
                id        TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;

        // create responses table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xresponses\" (
                author    TEXT,
                question  TEXT,
                content   TEXT,
                id        TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;

        // create comments table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xcomments\" (
                author    TEXT,
                response  TEXT,
                content   TEXT,
                id        TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;
    }

    // anonymous tag

    /// Get the tag of an anonymous username
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

    /// Create an anonymous username
    ///
    /// # Returns
    /// `("anonymous#" + tag, tag)`
    pub fn create_anonymous(&self) -> (String, String) {
        let tag = xsu_util::hash::random_id();
        (format!("anonymous#{tag}"), tag)
    }

    // profiles

    /// Fetch a profile correctly
    pub async fn get_profile(&self, mut id: String) -> Result<Profile> {
        if id.starts_with("ANSWERED:") {
            // we use the "ANSWERED" prefix whenever we answer a question so it doesn't show up in inboxes
            id = id.replace("ANSWERED:", "");
        }

        if id == "@" {
            return Ok(global_profile());
        } else if id.starts_with("anonymous#") | (id == "anonymous") | (id == "#") {
            let tag = Database::anonymous_tag(&id);
            return Ok(anonymous_profile(tag.3));
        }

        // handle legacy IDs (usernames)
        if id.len() <= 32 {
            return match self.auth.get_profile_by_username(id).await {
                Ok(ua) => Ok(ua),
                Err(_) => Err(DatabaseError::Other),
            };
        }

        match self.auth.get_profile_by_id(id).await {
            Ok(ua) => Ok(ua),
            Err(_) => Err(DatabaseError::Other),
        }
    }

    // questions

    /// Get an existing question
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_question(&self, id: String) -> Result<Question> {
        // legacy migration
        if id.starts_with("{") {
            let question = serde_json::from_str::<serde_json::Value>(&id).unwrap();

            return Ok(Question {
                author: match self
                    .get_profile(
                        question
                            .get("author")
                            .unwrap()
                            .to_string()
                            .trim_matches(|c| c == '"')
                            .to_string(),
                    )
                    .await
                {
                    Ok(ua) => ua,
                    Err(e) => return Err(e),
                },
                recipient: match self
                    .get_profile(
                        question
                            .get("recipient")
                            .unwrap()
                            .to_string()
                            .trim_matches(|c| c == '"')
                            .to_string(),
                    )
                    .await
                {
                    Ok(ua) => ua,
                    Err(e) => return Err(e),
                },
                content: question
                    .get("content")
                    .unwrap()
                    .to_string()
                    .trim_matches(|c| c == '"')
                    .to_string(),
                id: question
                    .get("id")
                    .unwrap()
                    .to_string()
                    .trim_matches(|c| c == '"')
                    .to_string(),
                timestamp: question
                    .get("timestamp")
                    .unwrap()
                    .to_string()
                    .trim_matches(|c| c == '"')
                    .parse::<u128>()
                    .unwrap(),
            });
        }

        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.question:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<RefQuestion>(c.as_str()) {
                Ok(q) => {
                    return Ok(Question {
                        author: match self.get_profile(q.author.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) LOSTQ A:UID {}", e.to_string(), q.author);
                                let tag = self.create_anonymous();
                                return Ok(Question::lost(tag));
                            }
                        },
                        recipient: match self.get_profile(q.recipient.clone()).await {
                            Ok(ua) => ua,
                            Err(e) => {
                                println!("({}) LOSTQ R:UID {}", e.to_string(), q.recipient);
                                let tag = self.create_anonymous();
                                return Ok(Question::lost(tag));
                            }
                        },
                        content: q.content,
                        id: q.id,
                        timestamp: q.timestamp,
                    })
                }
                Err(_) => {
                    // remove bad entry and continue to fetch from database
                    self.base
                        .cachedb
                        .remove(format!("xsulib.sparkler.question:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => {
                let tag = self.create_anonymous();
                return Ok(Question::lost(tag));
            }
        };

        // return
        let question = Question {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            recipient: match self
                .get_profile(res.get("recipient").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            content: res.get("content").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.question:{}", id),
                serde_json::to_string::<RefQuestion>(&RefQuestion::from(question.clone())).unwrap(),
            )
            .await;

        // return
        Ok(question)
    }

    /// Get all questions by their recipient
    ///
    /// ## Arguments:
    /// * `recipient`
    pub async fn get_questions_by_recipient(&self, recipient: String) -> Result<Vec<Question>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"recipient\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"recipient\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&recipient.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Question> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(Question {
                        author: match self
                            .get_profile(res.get("author").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(e) => return Err(e),
                        },
                        recipient: match self
                            .get_profile(res.get("recipient").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(e) => return Err(e),
                        },
                        content: res.get("content").unwrap().to_string(),
                        id: res.get("id").unwrap().to_string(),
                        timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_global_questions_by_author(
        &self,
        author: String,
    ) -> Result<Vec<(Question, i32)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = ? AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = $1 AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, i32)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: res.get("id").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.base
                            .cachedb
                            .get(format!("xsulib.sparkler.question_response_count:{}", id))
                            .await
                            .unwrap_or(String::from("0"))
                            .parse::<i32>()
                            .unwrap_or(0),
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by their author, 50 at a time
    ///
    /// ## Arguments:
    /// * `author`
    /// * `page`
    pub async fn get_global_questions_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<(Question, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = ? AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = $1 AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: res.get("id").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.base
                            .cachedb
                            .get(format!("xsulib.sparkler.question_response_count:{}", id))
                            .await
                            .unwrap_or(String::from("0"))
                            .parse::<usize>()
                            .unwrap_or(
                                self.get_responses_by_question(id)
                                    .await
                                    .unwrap_or(Vec::new())
                                    .len(),
                            ),
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get 50 global questions from people `user` is following
    ///
    /// ## Arguments:
    /// * `user`
    pub async fn get_global_questions_by_following(
        &self,
        user: String,
    ) -> Result<Vec<(Question, i32)>> {
        // get following
        let following = match self.auth.get_following(user.clone()).await {
            Ok(f) => f,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // check user permissions
        // returning NotAllowed here will block them from viewing their global questions timeline
        // we don't want to waste resources on rule breakers
        let user = match self.auth.get_profile_by_username(user.clone()).await {
            Ok(ua) => ua,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // build string
        let mut query_string = String::new();

        for follow in following {
            query_string.push_str(&format!(" OR \"author\" = '{}'", follow.2.id));
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            // we're also going to include our own responses so we don't have to do any complicated stuff to detect if we should start with "OR" (previous)
            format!("SELECT * FROM \"xquestions\" WHERE (\"author\" = ?{query_string}) AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 50")
        } else {
            format!( "SELECT * FROM \"xquestions\" WHERE (\"author\" = $1{query_string}) AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 50")
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user.id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, i32)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();
                    out.push((
                        Question {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => {
                                    println!(
                                        "({}) UID {}",
                                        e.to_string(),
                                        res.get("author").unwrap().to_string()
                                    );

                                    continue;
                                }
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => {
                                    println!(
                                        "({}) UID {}",
                                        e.to_string(),
                                        res.get("recipient").unwrap().to_string()
                                    );

                                    continue;
                                }
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.base
                            .cachedb
                            .get(format!("xsulib.sparkler.question_response_count:{}", id))
                            .await
                            .unwrap_or(String::from("0"))
                            .parse::<i32>()
                            .unwrap_or(0),
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get the number of global questions by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_global_questions_count_by_author(&self, author: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.global_questions_count:{}", author))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_global_questions_by_author(author.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.global_question_count:{}", author),
                count.to_string(),
            )
            .await;

        count
    }

    /// Create a new question
    ///
    /// ## Arguments:
    /// * `props` - [`QuestionCreate`]
    /// * `author` - the username of the user creating the question
    pub async fn create_question(&self, mut props: QuestionCreate, author: String) -> Result<()> {
        // check content length
        if props.content.trim().len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        if props.content.len() > 250 {
            return Err(DatabaseError::ContentTooLong);
        }

        // check recipient
        // "@" is the recipient we use for global questions (questions anybody can respond to)
        let tag = Database::anonymous_tag(&author);
        if props.recipient != "@" {
            let recipient = match self
                .auth
                .get_profile_by_username(props.recipient.clone())
                .await
            {
                Ok(ua) => ua,
                Err(_) => return Err(DatabaseError::NotFound),
            };

            let profile_locked = recipient
                .metadata
                .kv
                .get("sparkler:lock_profile")
                .unwrap_or(&"false".to_string())
                == "true";

            let block_anonymous = recipient
                .metadata
                .kv
                .get("sparkler:disallow_anonymous")
                .unwrap_or(&"false".to_string())
                == "true";

            let block_list =
                if let Some(block_list) = recipient.metadata.kv.get("sparkler:block_list") {
                    block_list.to_string()
                } else {
                    String::new()
                };

            if profile_locked {
                return Err(DatabaseError::NotAllowed);
            }

            if (block_anonymous == true) && (tag.0 == true) {
                return Err(DatabaseError::NotAllowed);
            }

            if tag.0 == false {
                let author = match self.get_profile(author.clone()).await {
                    Ok(ua) => ua,
                    Err(e) => return Err(e),
                };

                if block_list.contains(&format!("<@{}>", author.username)) {
                    return Err(DatabaseError::NotAllowed);
                }
            }
        } else {
            // anonymous users cannot ask global questions
            if tag.0 == true {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check author permissions
        if tag.0 == false {
            let author = match self.auth.get_profile_by_username(author.clone()).await {
                Ok(ua) => ua,
                Err(_) => return Err(DatabaseError::NotFound),
            };

            if author.group == -1 {
                // group -1 (even if it exists) is for marking users as banned
                return Err(DatabaseError::NotAllowed);
            }
        } else {
            // anonymous users cannot post images
            props.content = props.content.replace("![", "[").replace("<img", "<bimg");
        }

        // check markdown content
        let markdown = xsu_util::ui::render_markdown(&props.content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // ...
        let question = Question {
            author: match self.get_profile(author).await {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            recipient: match self.get_profile(props.recipient).await {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            content: props.content.trim().to_string(),
            id: utility::random_id(),
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xquestions\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xquestions\" VALEUS ($1, $2, $3, $4, $5)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&question.author.id)
            .bind::<&String>(&question.recipient.id)
            .bind::<&String>(&question.content)
            .bind::<&String>(&question.id)
            .bind::<&String>(&question.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // incr questions count
                if question.recipient.username == "@" {
                    self.base
                        .cachedb
                        .incr(format!(
                            "xsulib.sparkler.global_question_count:{}",
                            question.author.username
                        ))
                        .await;
                }

                // ...
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing question
    ///
    /// Questions can only be deleted by their recipient or the user that asked them.
    ///
    /// ## Arguments:
    /// * `id` - the ID of the question
    /// * `user` - the user doing this
    pub async fn delete_question(&self, id: String, user: Profile) -> Result<()> {
        // make sure question exists
        let question = match self.get_question(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        if (user.id != question.recipient.id) && (user.id != question.author.id) {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xquestions\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xquestions\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove all responses if this is a global question
                if question.recipient.username == "@" {
                    // delete responses
                    let query: String =
                        if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                            "DELETE FROM \"xresponses\" WHERE \"question\" LIKE ?"
                        } else {
                            "DELETE FROM \"xresponses\" WHERE \"question\" LIKE $1"
                        }
                        .to_string();

                    let c = &self.base.db.client;
                    if let Err(_) = sqlquery(&query)
                        .bind::<&String>(&format!("%\"id\":\"{}\"%", question.id))
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };

                    // delete response counter
                    self.base
                        .cachedb
                        .remove(format!(
                            "xsulib.sparkler.question_response_count:{}",
                            question.id
                        ))
                        .await;

                    // decr questions count
                    self.base
                        .cachedb
                        .decr(format!(
                            "xsulib.sparkler.global_question_count:{}",
                            question.author.username
                        ))
                        .await;
                }

                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("xsulib.sparkler.question:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // responses

    /// Get an existing response
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_response(&self, id: String) -> Result<(Question, QuestionResponse, usize)> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.response:{}", id))
            .await
        {
            Some(c) => {
                match serde_json::from_str::<(Question, QuestionResponse, usize)>(c.as_str()) {
                    Ok(r) => return Ok(r),
                    Err(_) => {
                        // we're storing a bad version that couldn't deserialize, we don't need that...
                        self.base
                            .cachedb
                            .remove(format!("xsulib.sparkler.response:{}", id))
                            .await
                    }
                };
            }
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let response = QuestionResponse {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            question: res.get("question").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.response:{}", id),
                serde_json::to_string::<QuestionResponse>(&response).unwrap(),
            )
            .await;

        // return
        Ok((
            match self.get_question(response.question.clone()).await {
                Ok(q) => q,
                Err(e) => return Err(e),
            },
            response,
            self.get_comment_count_by_response(id).await,
        ))
    }

    /// Get an existing response by the question ID and response author
    ///
    /// ## Arguments:
    /// * `question`
    /// * `author`
    pub async fn get_response_by_question_and_author(
        &self,
        question: String,
        author: String,
    ) -> Result<(Question, QuestionResponse, usize)> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = ? AND \"author\" = ?"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = $1 AND \"author\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&question)
            .bind::<&String>(&author)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let response = QuestionResponse {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            question: res.get("question").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // return
        Ok((
            match self.get_question(response.question.clone()).await {
                Ok(q) => q,
                Err(e) => return Err(e),
            },
            response,
            self.get_comment_count_by_response(question).await,
        ))
    }

    /// Get all responses by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_responses_by_author(
        &self,
        author: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let question = res.get("question").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        match self.get_question(question.clone()).await {
                            Ok(q) => q,
                            Err(e) => return Err(e),
                        },
                        QuestionResponse {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            question,
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        self.get_comment_count_by_response(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author, 50 at a time
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_responses_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<(Question, QuestionResponse, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let question = res.get("question").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        match self.get_question(question.clone()).await {
                            Ok(q) => q,
                            Err(e) => {
                                println!("({}) QID {}", e.to_string(), question);
                                continue;
                            }
                        },
                        QuestionResponse {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => {
                                    println!(
                                        "({}) UID {}",
                                        e.to_string(),
                                        res.get("author").unwrap().to_string()
                                    );

                                    continue;
                                }
                            },
                            question,
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        self.get_comment_count_by_response(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get the number of responses by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_response_count_by_author(&self, author: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.response_count:{}", author))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_responses_by_author(author.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.response_count:{}", author),
                count.to_string(),
            )
            .await;

        count
    }

    /// Get 50 responses from people `user` is following
    ///
    /// ## Arguments:
    /// * `user`
    pub async fn get_responses_by_following(
        &self,
        user: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize)>> {
        // get following
        let following = match self.auth.get_following(user.clone()).await {
            Ok(f) => f,
            Err(_) => Vec::new(),
        };

        // check user permissions
        // returning NotAllowed here will block them from viewing their timeline
        // we don't want to waste resources on rule breakers
        let user = match self.auth.get_profile_by_id(user.clone()).await {
            Ok(ua) => ua,
            Err(_) => anonymous_profile(self.create_anonymous().1),
        };

        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // build string
        let mut query_string = String::new();

        for follow in following {
            query_string.push_str(&format!(" OR \"author\" = '{}'", follow.2.id));
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            // we're also going to include our own responses so we don't have to do any complicated stuff to detect if we should start with "OR" (previous)
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ?{query_string} ORDER BY \"timestamp\" DESC LIMIT 50")
        } else {
            format!( "SELECT * FROM \"xresponses\" WHERE \"author\" = $1{query_string} ORDER BY \"timestamp\" DESC LIMIT 50")
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user.id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let question = res.get("question").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        match self.get_question(question.clone()).await {
                            Ok(q) => q,
                            Err(_) => {
                                let tag = self.create_anonymous();
                                Question::lost(tag)
                            }
                        },
                        QuestionResponse {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => {
                                    let tag = self.create_anonymous();
                                    anonymous_profile(tag.1)
                                }
                            },
                            question,
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        self.get_comment_count_by_response(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their question ID
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_responses_by_question(
        &self,
        id: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"question\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let question = res.get("question").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        match self.get_question(question.clone()).await {
                            Ok(q) => q,
                            Err(e) => return Err(e),
                        },
                        QuestionResponse {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(e) => return Err(e),
                            },
                            question,
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        self.get_comment_count_by_response(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Create a new response
    ///
    /// Responses can only be created for questions where `recipient` matches the given `author`
    ///
    /// ## Arguments:
    /// * `props` - [`ResponseCreate`]
    /// * `author` - the username of the user creating the response
    pub async fn create_response(&self, props: ResponseCreate, author: String) -> Result<()> {
        // make sure the question exists
        let question = match self.get_question(props.question.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check permissions
        if question.recipient.username != "@" {
            if question.recipient.id != author {
                // cannot respond to a question not asked to us
                return Err(DatabaseError::NotAllowed);
            }
        } else {
            // TODO: check author privacy settings
            let tag = Database::anonymous_tag(&author);

            if tag.0 {
                // anonymous users cannot answer global questions
                return Err(DatabaseError::NotAllowed);
            }

            // make sure we didn't already answer this
            if let Ok(_) = self
                .get_response_by_question_and_author(question.id.clone(), author.clone())
                .await
            {
                // cannot answer the same global question twice
                return Err(DatabaseError::NotAllowed);
            };
        };

        // check content length
        if props.content.len() > 1000 {
            return Err(DatabaseError::ContentTooLong);
        }

        if props.content.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check author permissions
        let author = match self.get_profile(author.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // check markdown content
        let markdown = xsu_util::ui::render_markdown(&props.content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // ...
        let response = QuestionResponse {
            author,
            question: question.id,
            content: props.content.trim().to_string(),
            id: utility::random_id(),
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xresponses\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xresponses\" VALEUS ($1, $2, $3, $4, $5)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&response.author.id)
            .bind::<&String>(&response.question)
            .bind::<&String>(&response.content)
            .bind::<&String>(&response.id)
            .bind::<&String>(&response.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification
                let tag = Database::anonymous_tag(&question.author.username);

                if (question.recipient.id != question.author.id) && tag.0 == false {
                    if let Err(_) = self
                        .auth
                        .create_notification(NotificationCreate {
                            title: format!(
                                "[@{}](/@{}) responded to a question you asked!",
                                response.author.username, response.author.username
                            ),
                            content: format!(
                                "{}: \"{}...\"",
                                response.author.username,
                                // we're only going to show 50 characters of the response in the notification
                                response
                                    .content
                                    .clone()
                                    .chars()
                                    .take(50)
                                    .collect::<String>()
                            ),
                            address: format!("/response/{}", response.id),
                            recipient: question.author.id,
                        })
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }

                // handle global questions
                if question.recipient.username == "@" {
                    // this is a global ask, we need to respond to it and then just move on

                    // bump question response count
                    self.base
                        .cachedb
                        .incr(format!(
                            "xsulib.sparkler.question_response_count:{}",
                            response.question
                        ))
                        .await;

                    // bump response count
                    self.base
                        .cachedb
                        .incr(format!(
                            "xsulib.sparkler.response_count:{}",
                            response.author.id
                        ))
                        .await;

                    return Ok(());
                } else {
                    // change recipient so it doesn't show up in inbox
                    let query: String =
                        if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                            "UPDATE \"xquestions\" SET \"recipient\" = ? WHERE \"id\" = ?"
                        } else {
                            "UPDATE \"xquestions\" SET (\"recipient\") = ($1) WHERE \"id\" = $2"
                        }
                        .to_string();

                    let c = &self.base.db.client;
                    if let Err(_) = sqlquery(&query)
                        .bind::<&String>(&format!("ANSWERED:{}", question.recipient.id))
                        .bind::<&String>(&response.question)
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    }
                }

                // bump response count
                self.base
                    .cachedb
                    .incr(format!(
                        "xsulib.sparkler.response_count:{}",
                        response.author.id
                    ))
                    .await;

                // return
                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Create a new response
    ///
    /// Responses can only be created for questions where `recipient` matches the given `author`
    ///
    /// ## Arguments:
    /// * `content`
    /// * `user` - the user doing this
    pub async fn update_response_content(
        &self,
        id: String,
        content: String,
        user: Profile,
    ) -> Result<()> {
        // make sure the response exists
        let response = match self.get_response(id.clone()).await {
            Ok(q) => q.0,
            Err(e) => return Err(e),
        };

        // check content length
        if content.len() > 1000 {
            return Err(DatabaseError::ContentTooLong);
        }

        if content.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != response.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check markdown content
        let markdown = xsu_util::ui::render_markdown(&content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // update response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xresponses\" SET \"content\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xresponses\" SET (\"content\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&content)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("xsulib.sparkler.response:{id}"))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Delete an existing question
    ///
    /// Responses can only be deleted by their author.
    ///
    /// ## Arguments:
    /// * `id` - the ID of the response
    /// * `user` - the user doing this
    pub async fn delete_response(&self, id: String, user: Profile) -> Result<()> {
        // make sure response exists
        let response = match self.get_response(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != response.1.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // delete question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xresponses\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xresponses\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("xsulib.sparkler.response:{}", id))
                    .await;

                // decr response count
                self.base
                    .cachedb
                    .decr(format!(
                        "xsulib.sparkler.response_count:{}",
                        response.1.author.id
                    ))
                    .await;

                // decr global question response count
                if response.0.recipient.username == "@" {
                    self.base
                        .cachedb
                        .decr(format!(
                            "xsulib.sparkler.question_response_count:{}",
                            response.0.id
                        ))
                        .await;
                }

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // responses

    /// Get an existing comment
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_comment(&self, id: String) -> Result<ResponseComment> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.comment:{}", id))
            .await
        {
            Some(c) => return Ok(serde_json::from_str::<ResponseComment>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let comment = ResponseComment {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(e) => return Err(e),
            },
            response: res.get("response").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.comment:{}", id),
                serde_json::to_string::<ResponseComment>(&comment).unwrap(),
            )
            .await;

        // return
        Ok(comment)
    }

    /// Get all comments by their response ID
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_comments_by_response(&self, id: String) -> Result<Vec<ResponseComment>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"response\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"response\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<ResponseComment> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(ResponseComment {
                        author: match self
                            .get_profile(res.get("author").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(e) => return Err(e),
                        },
                        response: res.get("response").unwrap().to_string(),
                        content: res.get("content").unwrap().to_string(),
                        id: res.get("id").unwrap().to_string(),
                        timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all comments by their response ID, 50 at a time
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_comments_by_response_paginated(
        &self,
        id: String,
        page: i32,
    ) -> Result<Vec<ResponseComment>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xcomments\" WHERE \"response\" = ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xcomments\" WHERE \"response\" = $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<ResponseComment> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(ResponseComment {
                        author: match self
                            .get_profile(res.get("author").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(e) => return Err(e),
                        },
                        response: res.get("response").unwrap().to_string(),
                        content: res.get("content").unwrap().to_string(),
                        id: res.get("id").unwrap().to_string(),
                        timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of comments by their response ID
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_comment_count_by_response(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.comment_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_comments_by_response(id.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.comment_count:{}", id),
                count.to_string(),
            )
            .await;

        count
    }

    /// Create a new comment
    ///
    /// Responses can only be created by non-anonymous users
    ///
    /// ## Arguments:
    /// * `props` - [`CommentCreate`]
    /// * `author` - the username of the user creating the comment
    pub async fn create_comment(&self, props: CommentCreate, author: String) -> Result<()> {
        // make sure the response exists
        let response = match self.get_response(props.response.clone()).await {
            Ok(q) => q.1,
            Err(e) => return Err(e),
        };

        let tag = Database::anonymous_tag(&author);

        if tag.0 {
            // anonymous users cannot comment
            return Err(DatabaseError::NotAllowed);
        }

        // check content length
        if props.content.len() > 500 {
            return Err(DatabaseError::ContentTooLong);
        }

        if props.content.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check author permissions
        let author = match self.auth.get_profile_by_username(author.clone()).await {
            Ok(ua) => ua,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // check markdown content
        let markdown = xsu_util::ui::render_markdown(&props.content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // ...
        let comment = ResponseComment {
            author,
            response: response.id.clone(),
            content: props.content.trim().to_string(),
            id: utility::random_id(),
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xcomments\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xcomments\" VALEUS ($1, $2, $3, $4, $5)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&comment.author.id)
            .bind::<&String>(&comment.response)
            .bind::<&String>(&comment.content)
            .bind::<&String>(&comment.id)
            .bind::<&String>(&comment.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification
                if response.author.id != comment.author.id {
                    if let Err(_) = self
                        .auth
                        .create_notification(NotificationCreate {
                            title: format!(
                                "[@{}](/@{}) commented on a response you created!",
                                comment.author.username, comment.author.username
                            ),
                            content: format!(
                                "{}: \"{}...\"",
                                comment.author.username,
                                // we're only going to show 50 characters of the response in the notification
                                comment.content.clone().chars().take(50).collect::<String>()
                            ),
                            address: format!("/comment/{}", comment.id),
                            recipient: response.author.id,
                        })
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }

                // bump comment count
                self.base
                    .cachedb
                    .incr(format!("xsulib.sparkler.comment_count:{}", response.id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing comment
    ///
    /// Comments can only be deleted by their author.
    ///
    /// ## Arguments:
    /// * `id` - the ID of the comment
    /// * `user` - the user doing this
    pub async fn delete_comment(&self, id: String, user: Profile) -> Result<()> {
        // make sure comment exists
        let comment = match self.get_comment(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != comment.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // delete question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xcomments\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xcomments\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("xsulib.sparkler.comment:{}", id))
                    .await;

                // decr response count
                self.base
                    .cachedb
                    .decr(format!(
                        "xsulib.sparkler.comment_count:{}",
                        comment.response
                    ))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }
}
