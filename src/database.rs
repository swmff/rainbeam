use async_recursion::async_recursion;
use std::collections::HashMap;

use crate::config::Config;
use crate::model::{
    anonymous_profile, global_profile, Circle, CircleCreate, CircleMetadata, CommentCreate,
    DataExport, MembershipStatus, QuestionCreate, QuestionResponse, Reaction, RefQuestion,
    RelationshipStatus, ResponseComment, ResponseContext, ResponseCreate,
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
                timestamp TEXT,
                ip        TEXT
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
                timestamp TEXT,
                tags      TEXT,
                context   TEXT
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
                timestamp TEXT,
                reply     TEXT
            )",
        )
        .execute(c)
        .await;

        // create reactions table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xreactions\" (
                user      TEXT,
                asset     TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;

        // create circles table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xcircles\" (
                name      TEXT,
                id        TEXT,
                owner     TEXT,
                metadata  TEXT,
                timestamp TEXT
            )",
        )
        .execute(c)
        .await;

        // create circle memberships table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xcircle_memberships\" (
                user       TEXT,
                circle     TEXT,
                membership TEXT,
                timestamp  TEXT
            )",
        )
        .execute(c)
        .await;

        // create relationships table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xrelationships\" (
                one        TEXT,
                two        TEXT,
                status     TEXT,
                timestamp  TEXT
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

    /// Get all profiles by a search query, 50 at a time
    ///
    /// ## Arguments:
    /// * `page`
    /// * `search`
    pub async fn get_profiles_searched_paginated(
        &self,
        page: i32,
        search: String,
    ) -> Result<Vec<Profile>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xprofiles\" WHERE \"username\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xprofiles\" WHERE \"username\" LIKE $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Profile> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();
                    out.push(match self.get_profile(id).await {
                        Ok(p) => p,
                        Err(_) => continue,
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Export all data of the given `user`
    pub async fn create_data_export(&self, user: String) -> Result<DataExport> {
        Ok(DataExport {
            profile: match self.get_profile(user.clone()).await {
                Ok(r) => r,
                Err(e) => return Err(e),
            },
            questions: match self.get_questions_by_author(user.clone()).await {
                Ok(r) => r,
                Err(e) => return Err(e),
            },
            responses: match self.get_responses_by_author(user.clone()).await {
                Ok(r) => r,
                Err(e) => return Err(e),
            },
            comments: match self.get_comments_by_author(user.clone()).await {
                Ok(r) => r,
                Err(e) => return Err(e),
            },
        })
    }

    // extra util

    /// Generate share content from 2 strings and a link
    pub fn share(
        host: &String,
        part_1: &String,
        part_2: &String,
        mut link: String,
        mut target_length: usize,
    ) -> String {
        link = format!("{host}{link}");

        // check chars
        // if anything takes up multiple characters then we cannot safely split the string
        // we're just going to return the link in this case
        for char in part_1.chars() {
            if char.len_utf8() != 1 {
                return link;
            }
        }

        for char in part_2.chars() {
            if char.len_utf8() != 1 {
                return link;
            }
        }

        // ...
        let link_size = link.len();
        target_length -= link_size;

        let mut out = String::new();
        let separator = " — ";

        let part_2_size = (target_length / 2) - 1;
        let sep_size = separator.len();
        let part_1_size = (target_length / 2) - sep_size;

        out += if part_1_size > part_1.len() {
            // just use part_1
            part_1
        } else {
            &part_1[..part_1_size]
        };

        out += separator;

        if !part_2.is_empty() {
            out += &if part_2_size > part_2.len() {
                // just use part_2
                part_2
            } else {
                &part_2[..part_2_size]
            }
        }

        out += &format!(" {link}");
        out
    }

    // questions

    /// Get an existing question
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_question(&self, id: String) -> Result<Question> {
        if id == "0" {
            return Ok(Question::post());
        }

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
                ip: question
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
        // we still prefix neospring under the "sparkler" name for compatibility with the first 6 development versions
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
                            Err(_) => anonymous_profile(q.author),
                        },
                        recipient: match self
                            .get_profile(if q.recipient.starts_with("ANSWERED:") {
                                q.recipient.replace("ANSWERED:", "")
                            } else {
                                q.recipient.clone()
                            })
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => anonymous_profile(q.recipient),
                        },
                        content: q.content,
                        id: q.id,
                        ip: q.ip,
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
            "SELECT * FROM \"xquestions\" WHERE \"id\" LIKE ?"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"id\" LIKE $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("{id}%"))
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Ok(Question::unknown()),
        };

        // return
        let question = Question {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile(res.get("author").unwrap().to_string()),
            },
            recipient: match self
                .get_profile(res.get("recipient").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile(res.get("recipient").unwrap().to_string()),
            },
            content: res.get("content").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            ip: res.get("ip").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // store in cache
        if id.len() == 64 {
            self.base
                .cachedb
                .set(
                    format!("xsulib.sparkler.question:{}", id),
                    serde_json::to_string::<RefQuestion>(&RefQuestion::from(question.clone()))
                        .unwrap(),
                )
                .await;
        }

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
                            Err(_) => anonymous_profile("anonymous".to_string()),
                        },
                        recipient: match self
                            .get_profile(res.get("recipient").unwrap().to_string())
                            .await
                        {
                            Ok(ua) => ua,
                            Err(_) => anonymous_profile("anonymous".to_string()),
                        },
                        content: res.get("content").unwrap().to_string(),
                        id: res.get("id").unwrap().to_string(),
                        ip: res.get("ip").unwrap().to_string(),
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

    /// Get all questions by their author, 50 at a time
    ///
    /// ## Arguments:
    /// * `author`
    /// * `page`
    pub async fn get_questions_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<Question>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
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
                            Err(_) => continue,
                        },
                        content: res.get("content").unwrap().to_string(),
                        id: res.get("id").unwrap().to_string(),
                        ip: res.get("ip").unwrap().to_string(),
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
    ) -> Result<Vec<(Question, usize, usize)>> {
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
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

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
                            id: id.clone(),
                            ip: res.get("ip").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
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
    ) -> Result<Vec<(Question, usize, usize)>> {
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
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

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
                            id: id.clone(),
                            ip: res.get("ip").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by their author and a search query, 50 at a time
    ///
    /// ## Arguments:
    /// * `author`
    /// * `search`
    /// * `page`
    pub async fn get_global_questions_by_author_searched_paginated(
        &self,
        author: String,
        search: String,
        page: i32,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = ? AND \"recipient\" = '@' AND \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"author\" = $1 AND \"recipient\" = '@' AND \"content\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

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
                            id: id.clone(),
                            ip: res.get("ip").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all global questions by a search query, 50 at a time
    ///
    /// ## Arguments:
    /// * `page`
    /// * `search`
    pub async fn get_global_questions_searched_paginated(
        &self,
        page: i32,
        search: String,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' AND \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' AND \"content\" LIKE $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

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
                            id: id.clone(),
                            ip: res.get("ip").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get global questions from people `user` is following, 50 at a time
    ///
    /// # Arguments
    /// * `user`
    /// * `page`
    pub async fn get_global_questions_by_following_paginated(
        &self,
        user: String,
        page: i32,
    ) -> Result<Vec<(Question, usize, usize)>> {
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
            format!("SELECT * FROM \"xquestions\" WHERE (\"author\" = ?{query_string}) AND \"recipient\" = '@'  ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!( "SELECT * FROM \"xquestions\" WHERE (\"author\" = $1{query_string}) AND \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user.id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

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
                            id: id.clone(),
                            ip: res.get("ip").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all global questions, 50 at a time
    ///
    /// # Arguments
    /// * `page`
    pub async fn get_global_questions_paginated(
        &self,
        page: i32,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            // we're also going to include our own responses so we don't have to do any complicated stuff to detect if we should start with "OR" (previous)
            format!("SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!( "SELECT * FROM \"xquestions\" WHERE \"recipient\" = '@' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

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
                            id: id.clone(),
                            ip: res.get("ip").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
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
    /// # Arguments
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

    /// Get all questions by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_questions_by_author(
        &self,
        author: String,
    ) -> Result<Vec<(Question, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xquestions\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, usize, usize)> = Vec::new();

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
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            recipient: match self
                                .get_profile(res.get("recipient").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => continue,
                            },
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            ip: res.get("ip").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.get_response_count_by_question(id.clone()).await,
                        // get the number of reactions the question has
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of responses by their question
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_response_count_by_question(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.question_response_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_responses_by_question(id.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.question_response_count:{}", id),
                count.to_string(),
            )
            .await;

        count
    }

    /// Create a new question
    ///
    /// # Arguments
    /// * `props` - [`QuestionCreate`]
    /// * `author` - the ID of the user creating the question
    /// * `ip` - author IP
    pub async fn create_question(
        &self,
        mut props: QuestionCreate,
        author: String,
        ip: String,
    ) -> Result<()> {
        // check content length
        if props.content.trim().len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        if props.content.len() > (64 * 32) {
            return Err(DatabaseError::ContentTooLong);
        }

        // check recipient
        // "@" is the recipient we use for global questions (questions anybody can respond to)
        let tag = Database::anonymous_tag(&author);
        if props.recipient != "@" {
            if props.recipient.starts_with("@") {
                // circle
                let circle_name = props.recipient.replacen("@", "", 1);

                let circle = match self.get_circle_by_name(circle_name).await {
                    Ok(c) => c,
                    Err(e) => return Err(e),
                };

                let profile_locked = circle
                    .metadata
                    .kv
                    .get("sparkler:lock_profile")
                    .unwrap_or(&"false".to_string())
                    == "true";

                let block_anonymous = circle
                    .metadata
                    .kv
                    .get("sparkler:disallow_anonymous")
                    .unwrap_or(&"false".to_string())
                    == "true";

                if profile_locked {
                    return Err(DatabaseError::NotAllowed);
                }

                if (block_anonymous == true) && (tag.0 == true) {
                    return Err(DatabaseError::NotAllowed);
                }
            } else {
                // profile
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

                    let relationship = self
                        .get_user_relationship(author.id.clone(), recipient.id.clone())
                        .await
                        .0;

                    if relationship == RelationshipStatus::Blocked {
                        return Err(DatabaseError::NotAllowed);
                    }
                }

                // check filter
                for filter_string in recipient
                    .metadata
                    .kv
                    .get("sparkler:filter")
                    .unwrap_or(&"".to_string())
                    .split("\n")
                {
                    if filter_string.is_empty() | filter_string.starts_with("#") {
                        continue;
                    }

                    if props.content.contains(filter_string) {
                        return Err(DatabaseError::Filtered);
                    }
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
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            recipient: match self.get_profile(props.recipient.clone()).await {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            content: props.content.trim().to_string(),
            id: utility::random_id(),
            timestamp: utility::unix_epoch_timestamp(),
            ip: ip.clone(),
        };

        // create question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xquestions\" VALUES (?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xquestions\" VALEUS ($1, $2, $3, $4, $5, $6)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&question.author.id)
            .bind::<&String>(&props.recipient) // circles will have anonymous as the recipient
            .bind::<&String>(&question.content)
            .bind::<&String>(&question.id)
            .bind::<&String>(&question.timestamp.to_string())
            .bind::<&String>(&ip)
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
    /// # Arguments
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
            if question.recipient.id.starts_with("@") && question.recipient.id != "@" {
                // circles
                let circle_name = question.recipient.id.replacen("@", "", 1);

                let circle = match self.get_circle_by_name(circle_name).await {
                    Ok(c) => c,
                    Err(e) => return Err(e),
                };

                // check circle membership
                let membership = self
                    .get_user_circle_membership(user.id.clone(), circle.id.clone())
                    .await;

                if membership != MembershipStatus::Active {
                    return Err(DatabaseError::NotAllowed);
                }
            } else {
                // check permission
                let group = match self.auth.get_group_by_id(user.group).await {
                    Ok(g) => g,
                    Err(_) => return Err(DatabaseError::Other),
                };

                if !group.permissions.contains(&Permission::Helper) {
                    return Err(DatabaseError::NotAllowed);
                }
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

                    // clear reactions
                    if let Err(e) = self.clear_reactions(question.id).await {
                        return Err(e);
                    }
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

    /// Get a response from a database result
    pub async fn gimme_response(
        &self,
        res: HashMap<String, String>,
    ) -> Result<(Question, QuestionResponse, usize, usize)> {
        let question = res.get("question").unwrap().to_string();
        let id = res.get("id").unwrap().to_string();
        let author = res.get("author").unwrap().to_string();
        let ctx: ResponseContext =
            match serde_json::from_str(res.get("context").unwrap_or(&"{}".to_string())) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            };

        Ok((
            if ctx.is_post {
                // don't even try to fetch question, it doesn't exist
                Question::unknown()
            } else {
                match self.get_question(question.clone()).await {
                    Ok(q) => q,
                    Err(_) => Question::unknown(),
                }
            },
            QuestionResponse {
                author: if author.starts_with("{") {
                    // likely serialized author struct
                    let de: Profile = serde_json::from_str(&author).unwrap();

                    match self.get_profile(de.id).await {
                        Ok(ua) => ua,
                        Err(_) => anonymous_profile("anonymous".to_string()),
                    }
                } else {
                    // must just be id, fetch normally
                    match self.get_profile(author).await {
                        Ok(ua) => ua,
                        Err(_) => anonymous_profile("anonymous".to_string()),
                    }
                },
                question,
                content: res.get("content").unwrap().to_string(),
                id: id.clone(),
                timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                tags: match serde_json::from_str(res.get("tags").unwrap()) {
                    Ok(t) => t,
                    Err(_) => return Err(DatabaseError::ValueError),
                },
                context: ctx,
            },
            self.get_comment_count_by_response(id.clone()).await,
            self.get_reaction_count_by_asset(id).await,
        ))
    }

    /// Get an existing response
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_response(
        &self,
        id: String,
    ) -> Result<(Question, QuestionResponse, usize, usize)> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.response:{}", id))
            .await
        {
            Some(c) => {
                match serde_json::from_str::<HashMap<String, String>>(c.as_str()) {
                    Ok(res) => {
                        return Ok(match self.gimme_response(res).await {
                            Ok(r) => r,
                            Err(e) => return Err(e),
                        })
                    }
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
            "SELECT * FROM \"xresponses\" WHERE \"id\" LIKE ?"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"id\" LIKE $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("{id}%"))
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let response = match self.gimme_response(res).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        // store in cache
        if id.len() == 64 {
            self.base
                .cachedb
                .set(
                    format!("xsulib.sparkler.response:{}", id),
                    serde_json::to_string::<QuestionResponse>(&response.1).unwrap(),
                )
                .await;
        }

        // return
        Ok(response)
    }

    /// Get an existing response by the question ID and response author
    ///
    /// # Arguments
    /// * `question`
    /// * `author`
    pub async fn get_response_by_question_and_author(
        &self,
        question: String,
        author: String,
    ) -> Result<(Question, QuestionResponse, usize, usize)> {
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
        Ok(match self.gimme_response(res).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        })
    }

    /// Get all posts, 50 at a time
    ///
    /// # Arguments
    /// * `page`
    pub async fn get_posts_paginated(
        &self,
        page: i32,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" LIKE '%\"is_post\":true%' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" LIKE '%\"is_post\":true%' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all posts from users the user is following, 50 at a time
    ///
    /// # Arguments
    /// * `page`
    /// * `user`
    pub async fn get_posts_by_following_paginated(
        &self,
        page: i32,
        user: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
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
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" LIKE '%\"is_post\":true%' AND (\"author\" = ?{query_string}) ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" LIKE '%\"is_post\":true%' AND (\"author\" = $1{query_string}) ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind(&user.id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all responses, 50 at a time, matching search query
    ///
    /// # Arguments
    /// * `page`
    /// * `search`
    pub async fn get_posts_searched_paginated(
        &self,
        page: i32,
        search: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" LIKE '%\"is_post\":true%' AND \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"context\" LIKE '%\"is_post\":true%' AND \"content\" LIKE $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_responses_by_author(
        &self,
        author: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
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
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
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
    /// # Arguments
    /// * `author`
    pub async fn get_responses_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
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
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author and content search, 50 at a time
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_responses_by_author_searched_paginated(
        &self,
        author: String,
        search: String,
        page: i32,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ? AND \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = $1 AND \"content\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their author and tag, 50 at a time
    ///
    /// # Arguments
    /// * `author`
    /// * `tag`
    pub async fn get_responses_by_author_tagged_paginated(
        &self,
        author: String,
        tag: String,
        page: i32,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ? AND \"tags\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = $1 AND \"tags\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .bind::<&String>(&format!("%\"{}\"%", tag))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their tag, 50 at a time
    ///
    /// # Arguments
    /// * `author`
    /// * `tag`
    pub async fn get_responses_tagged_paginated(
        &self,
        tag: String,
        page: i32,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"tags\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"tags\" LIKE $2 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%\"{}\"%", tag))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
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
    /// # Arguments
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

    /// Get all responses, 50 at a time, matching search query
    ///
    /// # Arguments
    /// * `page`
    /// * `search`
    pub async fn get_responses_searched_paginated(
        &self,
        page: i32,
        search: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"content\" LIKE ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"content\" LIKE $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%{search}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get 50 responses from people `user` is following
    ///
    /// # Arguments
    /// * `user`
    pub async fn get_responses_by_following(
        &self,
        user: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
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
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
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
    /// # Arguments
    /// * `id`
    pub async fn get_responses_by_question(
        &self,
        id: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
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
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
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
    /// # Arguments
    /// * `props` - [`ResponseCreate`]
    /// * `author` - the ID of the user creating the response
    pub async fn create_response(&self, props: ResponseCreate, author: String) -> Result<()> {
        // make sure the question exists
        let question = if props.question != "0" {
            // get question from database
            match self.get_question(props.question.clone()).await {
                Ok(q) => q,
                Err(_) => Question::unknown(),
            }
        } else {
            // create post question
            Question::post()
        };

        // check content length
        if props.content.len() > (64 * 64) {
            return Err(DatabaseError::ContentTooLong);
        }

        if props.content.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        // check author permissions
        let mut author = match self.get_profile(author.clone()).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // check permissions
        if props.question != "0" {
            // normal questions
            if question.recipient.username != "@" {
                if !question.recipient.id.starts_with("@") {
                    if question.recipient.id != author.id {
                        // cannot respond to a question not asked to us
                        return Err(DatabaseError::NotAllowed);
                    }
                } else {
                    // circles
                    let circle_name = question.recipient.id.replacen("@", "", 1);

                    let circle = match self.get_circle_by_name(circle_name).await {
                        Ok(c) => c,
                        Err(e) => return Err(e),
                    };

                    // check circle membership
                    let membership = self
                        .get_user_circle_membership(author.id.clone(), circle.id.clone())
                        .await;

                    if membership != MembershipStatus::Active {
                        return Err(DatabaseError::NotAllowed);
                    }

                    // update author id
                    author.id = format!("{}%{}", author.id, circle.id); // tag with circle id
                }
            }
            // global questions
            else {
                // TODO: check author privacy settings
                let tag = Database::anonymous_tag(&author.id);

                if tag.0 {
                    // anonymous users cannot answer global questions
                    return Err(DatabaseError::NotAllowed);
                }

                // make sure we didn't already answer this
                if let Ok(_) = self
                    .get_response_by_question_and_author(question.id.clone(), author.id.clone())
                    .await
                {
                    // cannot answer the same global question twice
                    return Err(DatabaseError::NotAllowed);
                };
            };
        } else {
            // check tag
            let tag = Database::anonymous_tag(&author.id);

            if tag.0 {
                // anonymous users cannot create posts
                return Err(DatabaseError::NotAllowed);
            }
        };

        // check markdown content
        let markdown = xsu_util::ui::render_markdown(&props.content);

        if markdown.trim().len() == 0 {
            return Err(DatabaseError::ContentTooShort);
        }

        // ...
        let response = QuestionResponse {
            author,
            content: props.content.trim().to_string(),
            id: utility::random_id(),
            timestamp: utility::unix_epoch_timestamp(),
            tags: Vec::new(),
            context: ResponseContext {
                is_post: question.id == "0",
                warning: props.warning,
            },
            question: question.id,
        };

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xresponses\" VALUES (?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xresponses\" VALEUS ($1, $2, $3, $4, $5, $6, $7)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&response.author.id)
            .bind::<&String>(&response.question)
            .bind::<&String>(&response.content)
            .bind::<&String>(&response.id)
            .bind::<&String>(&response.timestamp.to_string())
            .bind::<&str>(&serde_json::to_string(&props.tags).unwrap_or("[]".to_string()))
            .bind::<&String>(&match serde_json::to_string(&response.context) {
                Ok(s) => s,
                Err(_) => return Err(DatabaseError::ValueError),
            })
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

                if (question.recipient.id != "@") && question.recipient.id.starts_with("@") {
                    // circle
                    let circle_name = question.recipient.id.replacen("@", "", 1);

                    let circle = match self.get_circle_by_name(circle_name).await {
                        Ok(c) => c,
                        Err(e) => return Err(e),
                    };

                    // bump response count
                    self.base
                        .cachedb
                        .incr(format!("xsulib.sparkler.response_count:{}", circle.id))
                        .await;
                } else {
                    // bump response count
                    self.base
                        .cachedb
                        .incr(format!(
                            "xsulib.sparkler.response_count:{}",
                            response.author.id
                        ))
                        .await;
                }

                // return
                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update an existing response's content
    ///
    /// # Arguments
    /// * `id`
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
            Ok(q) => q.1,
            Err(e) => return Err(e),
        };

        // check time
        let now = xsu_util::unix_epoch_timestamp();
        let diff = now - response.timestamp;
        let twenty_four_hours = 14400000;

        if diff >= twenty_four_hours {
            return Err(DatabaseError::OutOfTime);
        }

        // check content length
        if content.len() > 4096 {
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

    /// Update an existing response's tags
    ///
    /// # Arguments
    /// * `id`
    /// * `tags`
    /// * `user` - the user doing this
    pub async fn update_response_tags(
        &self,
        id: String,
        tags: Vec<String>,
        user: Profile,
    ) -> Result<()> {
        // make sure the response exists
        let response = match self.get_response(id.clone()).await {
            Ok(q) => q.1,
            Err(e) => return Err(e),
        };

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

        // update response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xresponses\" SET \"tags\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xresponses\" SET (\"tags\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&match serde_json::to_string(&tags) {
                Ok(t) => t,
                Err(_) => return Err(DatabaseError::ValueError),
            })
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

    /// Delete an existing response
    ///
    /// Responses can only be deleted by their author.
    ///
    /// # Arguments
    /// * `id` - the ID of the response
    /// * `user` - the user doing this
    /// * `save_question` - if we should not delete the question too
    pub async fn delete_response(
        &self,
        id: String,
        user: Profile,
        save_question: bool,
    ) -> Result<()> {
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

            if !group.permissions.contains(&Permission::Helper) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // delete response
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
                } else if !save_question {
                    // delete question
                    if let Err(e) = self
                        .delete_question(response.0.id, response.0.recipient)
                        .await
                    {
                        return Err(e);
                    };
                }

                // clear reactions
                if let Err(e) = self.clear_reactions(id).await {
                    return Err(e);
                }

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Return a response's question to the inbox and delete the response
    ///
    /// # Arguments
    /// * `id`
    /// * `user` - the user doing this
    pub async fn unsend_response(&self, id: String, user: Profile) -> Result<()> {
        // make sure the response exists
        let res = match self.get_response(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        let question = res.0;
        let response = res.1;

        if response.context.is_post {
            return Err(DatabaseError::Other);
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

        // update question
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xquestions\" SET \"recipient\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xquestions\" SET (\"recipient\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&question.recipient.id)
            .bind::<&String>(&question.id)
            .execute(c)
            .await
        {
            Ok(_) => {
                if let Err(e) = self.delete_response(response.id, user, true).await {
                    return Err(e);
                }

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    // comments

    /// Get an existing comment
    ///
    /// # Arguments
    /// * `id`
    /// * `recurse` - should be FALSE when fetching counts to prevent a stack overflow
    #[async_recursion]
    pub async fn get_comment(
        &self,
        id: String,
        recurse: bool,
    ) -> Result<(ResponseComment, usize, usize)> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.comment:{}", id))
            .await
        {
            Some(c) => {
                return Ok((
                    serde_json::from_str::<ResponseComment>(c.as_str()).unwrap(),
                    if recurse == true {
                        self.get_reply_count_by_comment(id.clone()).await
                    } else {
                        0
                    },
                    self.get_reaction_count_by_asset(id).await,
                ))
            }
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"id\" LIKE ?"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"id\" LIKE $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("{id}%"))
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let reply = res.get("reply").unwrap().to_string();
        let comment = ResponseComment {
            author: match self
                .get_profile(res.get("author").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            response: res.get("response").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
            reply: if reply.is_empty() {
                None
            } else {
                match Box::pin(self.get_comment(reply, recurse)).await {
                    Ok(r) => Some(Box::new(r.0)),
                    Err(_) => None,
                }
            },
        };

        // store in cache
        if id.len() == 64 {
            self.base
                .cachedb
                .set(
                    format!("xsulib.sparkler.comment:{}", id),
                    serde_json::to_string::<ResponseComment>(&comment).unwrap(),
                )
                .await;
        }

        // return
        Ok((
            comment,
            if recurse == true {
                self.get_reply_count_by_comment(id.clone()).await
            } else {
                0
            },
            self.get_reaction_count_by_asset(id).await,
        ))
    }

    /// Get all comments by their response ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_comments_by_response(
        &self,
        id: String,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"response\" LIKE ? AND \"reply\" = '' ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"response\" LIKE $1 AND \"reply\" = '' ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("{id}%"))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
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
    /// # Arguments
    /// * `id`
    pub async fn get_comments_by_response_paginated(
        &self,
        id: String,
        page: i32,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xcomments\" WHERE \"response\" = ? AND \"reply\" = '' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xcomments\" WHERE \"response\" = $1 AND \"reply\" = '' ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
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
    /// # Arguments
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

    /// Get all comments by their author ID
    ///
    /// # Arguments
    /// * `user`
    pub async fn get_comments_by_author(
        &self,
        user: String,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"author\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"author\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all replies by their comment ID
    ///
    /// # Arguments
    /// * `id`
    /// * `recurse` - should be FALSE when fetching counts to prevent a stack overflow
    #[async_recursion]
    pub async fn get_replies_by_comment(
        &self,
        id: String,
        recurse: bool,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcomments\" WHERE \"reply\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xcomments\" WHERE \"reply\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, recurse).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                        },
                        if recurse == true {
                            self.get_reply_count_by_comment(id.clone()).await
                        } else {
                            0
                        },
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all replies by their comment ID, 50 at a time
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_replies_by_comment_paginated(
        &self,
        id: String,
        page: i32,
    ) -> Result<Vec<(ResponseComment, usize, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xcomments\" WHERE \"reply\" = ? ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xcomments\" WHERE \"reply\" = $1 ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(ResponseComment, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;

                    let reply = res.get("reply").unwrap().to_string();
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        ResponseComment {
                            author: match self
                                .get_profile(res.get("author").unwrap().to_string())
                                .await
                            {
                                Ok(ua) => ua,
                                Err(_) => anonymous_profile("anonymous".to_string()),
                            },
                            response: res.get("response").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                            reply: if reply.is_empty() {
                                None
                            } else {
                                match self.get_comment(reply, true).await {
                                    Ok(r) => Some(Box::new(r.0)),
                                    Err(_) => None,
                                }
                            },
                        },
                        self.get_reply_count_by_comment(id.clone()).await,
                        self.get_reaction_count_by_asset(id).await,
                    ));
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get the number of replies by their comment ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_reply_count_by_comment(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.reply_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_replies_by_comment(id.clone(), false)
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.reply_count:{}", id),
                count.to_string(),
            )
            .await;

        count
    }

    /// Create a new comment
    ///
    /// Comments can only be created by non-anonymous users.
    ///
    /// # Arguments
    /// * `props` - [`CommentCreate`]
    /// * `author` - the ID of the user creating the comment
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
        if props.content.len() > (64 * 32) {
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
            reply: None,
        };

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xcomments\" VALUES (?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xcomments\" VALEUS ($1, $2, $3, $4, $5, $6)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&comment.author.id)
            .bind::<&String>(&comment.response)
            .bind::<&String>(&comment.content)
            .bind::<&String>(&comment.id)
            .bind::<&String>(&comment.timestamp.to_string())
            .bind::<&String>(&props.reply)
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification
                if !props.reply.is_empty() {
                    // send notification
                    let reply = match self.get_comment(props.reply.clone(), false).await {
                        Ok(r) => r.0,
                        Err(e) => return Err(e),
                    };

                    if reply.author.id != comment.author.id {
                        if let Err(_) = self
                            .auth
                            .create_notification(NotificationCreate {
                                title: format!(
                                    "[@{}](/@{}) replied to a comment you created!",
                                    comment.author.username, comment.author.username
                                ),
                                content: format!(
                                    "{}: \"{}...\"",
                                    comment.author.username,
                                    // we're only going to show 50 characters of the response in the notification
                                    comment.content.clone().chars().take(50).collect::<String>()
                                ),
                                address: format!("/comment/{}", comment.id),
                                recipient: reply.author.id,
                            })
                            .await
                        {
                            return Err(DatabaseError::Other);
                        };
                    }

                    // bump reply count
                    self.base
                        .cachedb
                        .incr(format!("xsulib.sparkler.reply_count:{}", props.reply))
                        .await;
                } else if response.author.id != comment.author.id {
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
    /// # Arguments
    /// * `id` - the ID of the comment
    /// * `user` - the user doing this
    pub async fn delete_comment(&self, id: String, user: Profile) -> Result<()> {
        // make sure comment exists
        let comment = match self.get_comment(id.clone(), false).await {
            Ok(q) => q.0,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != comment.author.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Helper) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // delete comment
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

                // decr reply count
                if comment.reply.is_some() {
                    self.base
                        .cachedb
                        .incr(format!("xsulib.sparkler.reply_count:{}", comment.id))
                        .await;
                }

                // clear reactions
                if let Err(e) = self.clear_reactions(id).await {
                    return Err(e);
                }

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // reactions

    /// Get an existing reaction
    ///
    /// # Arguments
    /// * `user` - the ID of the user
    /// * `asset` - the ID of the asset
    pub async fn get_reaction(&self, user: String, asset: String) -> Result<Reaction> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.reaction:{}:{}", user, asset))
            .await
        {
            Some(c) => return Ok(serde_json::from_str::<Reaction>(c.as_str()).unwrap()),
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xreactions\" WHERE \"user\" = ? AND \"asset\" = ?"
        } else {
            "SELECT * FROM \"xreactions\" WHERE \"user\" = $1 AND \"asset\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&asset)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let reaction = Reaction {
            user: match self.get_profile(res.get("user").unwrap().to_string()).await {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            asset: res.get("asset").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.reaction:{}:{}", user, asset),
                serde_json::to_string::<Reaction>(&reaction).unwrap(),
            )
            .await;

        // return
        Ok(reaction)
    }

    /// Get all reactions by their asset ID
    ///
    /// # Arguments
    /// * `asset`
    pub async fn get_reactions_by_asset(&self, id: String) -> Result<Vec<Reaction>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xreactions\" WHERE \"asset\" = ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xreactions\" WHERE \"asset\" = $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out: Vec<Reaction> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(Reaction {
                        user: match self.get_profile(res.get("user").unwrap().to_string()).await {
                            Ok(ua) => ua,
                            Err(_) => continue,
                        },
                        asset: res.get("asset").unwrap().to_string(),
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

    /// Get the number of reactions by their asset ID
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_reaction_count_by_asset(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.reaction_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_reactions_by_asset(id.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.reaction_count:{}", id),
                count.to_string(),
            )
            .await;

        count
    }

    /// Create a new reaction
    ///
    /// Reactions can only be created by non-anonymous users.
    ///
    /// # Arguments
    /// * `id` - the ID of the asset
    /// * `author` - the user creating the reaction
    pub async fn create_reaction(&self, id: String, author: Profile) -> Result<()> {
        let tag = Database::anonymous_tag(&author.username);

        if tag.0 {
            // anonymous users cannot comment
            return Err(DatabaseError::NotAllowed);
        }

        // make sure reaction doesn't already exist
        if let Ok(_) = self.get_reaction(author.id.clone(), id.clone()).await {
            return Err(DatabaseError::NotAllowed);
        }

        // check author permissions
        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // ...
        let reaction = Reaction {
            user: author,
            asset: id,
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create response
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xreactions\" VALUES (?, ?, ?)"
        } else {
            "INSERT INTO \"xreactions\" VALEUS ($1, $2, $3)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&reaction.user.id)
            .bind::<&String>(&reaction.asset)
            .bind::<&String>(&reaction.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // bump reaction count
                self.base
                    .cachedb
                    .incr(format!("xsulib.sparkler.reaction_count:{}", reaction.asset))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing reaction
    ///
    /// Reactions can only be deleted by their author.
    ///
    /// # Arguments
    /// * `id` - the ID of the reaction
    /// * `user` - the user doing this
    pub async fn delete_reaction(&self, id: String, user: Profile) -> Result<()> {
        // make sure reaction exists
        let reaction = match self.get_reaction(user.id.clone(), id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check username
        if user.id != reaction.user.id {
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

        // delete reaction
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xreactions\" WHERE \"user\" = ? AND \"asset\" = ?"
        } else {
            "DELETE FROM \"xreactions\" WHERE \"user\" = $1 AND \"asset\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&user.id)
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                // remove from cache
                self.base
                    .cachedb
                    .remove(format!("xsulib.sparkler.reaction:{}:{}", user.id, id))
                    .await;

                // decr response count
                self.base
                    .cachedb
                    .decr(format!("xsulib.sparkler.reaction_count:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete all reactions by their asset
    ///
    /// # Arguments
    /// * `id` - the ID of the asset
    pub async fn clear_reactions(&self, id: String) -> Result<()> {
        // delete reactions
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xreactions\" WHERE \"asset\" = ?"
        } else {
            "DELETE FROM \"xreactions\" WHERE \"asset\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                // clear reaction count
                self.base
                    .cachedb
                    .decr(format!("xsulib.sparkler.reaction_count:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    // circles

    /// Get an existing circle
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_circle(&self, id: String) -> Result<Circle> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.circle:{}", id))
            .await
        {
            Some(c) => match serde_json::from_str::<Circle>(c.as_str()) {
                Ok(c) => return Ok(c),
                Err(_) => {
                    self.base
                        .cachedb
                        .remove(format!("xsulib.sparkler.circle:{}", id))
                        .await;
                }
            },
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcircles\" WHERE \"id\" = ?"
        } else {
            "SELECT * FROM \"xcircles\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&id).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let circle = Circle {
            name: res.get("name").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            owner: match self
                .get_profile(res.get("owner").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            metadata: serde_json::from_str(res.get("metadata").unwrap()).unwrap(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.circle:{}", id),
                serde_json::to_string::<Circle>(&circle).unwrap(),
            )
            .await;

        // return
        Ok(circle)
    }

    /// Get an existing circle by name
    ///
    /// # Arguments
    /// * `name`
    pub async fn get_circle_by_name(&self, name: String) -> Result<Circle> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcircles\" WHERE \"name\" = ?"
        } else {
            "SELECT * FROM \"xcircles\" WHERE \"name\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&name).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let circle = Circle {
            name: res.get("name").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            owner: match self
                .get_profile(res.get("owner").unwrap().to_string())
                .await
            {
                Ok(ua) => ua,
                Err(_) => anonymous_profile("anonymous".to_string()),
            },
            metadata: serde_json::from_str(res.get("metadata").unwrap()).unwrap(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // return
        Ok(circle)
    }

    /// Get the membership status of the given user ID in the given circle ID
    ///
    /// # Arguments
    /// * `user`
    /// * `circle`
    pub async fn get_user_circle_membership(
        &self,
        user: String,
        circle: String,
    ) -> MembershipStatus {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcircle_memberships\" WHERE \"user\" = ? AND \"circle\" = ?"
        } else {
            "SELECT * FROM \"xcircle_memberships\" WHERE \"user\" = $1 AND \"circle\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&user)
            .bind::<&String>(&circle)
            .fetch_one(c)
            .await
        {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return MembershipStatus::Inactive,
        };

        // return
        serde_json::from_str(&res.get("membership").unwrap()).unwrap()
    }

    /// Get the membership status of the given user ID in all circles they are `Active` in
    ///
    /// # Arguments
    /// * `user`
    pub async fn get_user_circle_memberships(&self, user: String) -> Result<Vec<Circle>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcircle_memberships\" WHERE \"user\" = ? AND \"membership\" = '\"Active\"'"
        } else {
            "SELECT * FROM \"xcircle_memberships\" WHERE \"user\" = $1 AND \"membership\" = '\"Active\"'"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&user).fetch_all(c).await {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;

                    // get circle
                    let circle = match self
                        .get_circle(res.get("circle").unwrap().to_string())
                        .await
                    {
                        Ok(c) => c,
                        Err(e) => return Err(e),
                    };

                    // add to out
                    out.push(circle);
                }

                Ok(out)
            }
            Err(_) => return Err(DatabaseError::Other),
        }
    }

    /// Get all users in the given `circle`
    ///
    /// # Arguments
    /// * `circle`
    pub async fn get_circle_memberships(&self, id: String) -> Result<Vec<Profile>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xcircle_memberships\" WHERE \"circle\" = ? AND \"membership\" = '\"Active\"'"
        } else {
            "SELECT * FROM \"xcircle_memberships\" WHERE \"circle\" = $1 AND \"membership\" = '\"Active\"'"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).fetch_all(c).await {
            Ok(p) => {
                let mut out = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;

                    // get profile
                    let profile = match self.get_profile(res.get("user").unwrap().to_string()).await
                    {
                        Ok(c) => c,
                        Err(e) => return Err(e),
                    };

                    // add to out
                    out.push(profile);
                }

                Ok(out)
            }
            Err(_) => return Err(DatabaseError::Other),
        }
    }

    /// Get the number of memberships a circle has
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_circle_memberships_count(&self, id: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.circle_memberships_count:{}", id))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_circle_memberships(id.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.circle_memberships_count:{}", id),
                count.to_string(),
            )
            .await;

        count
    }

    /// Set the membership of `user` in the given `circle`
    ///
    /// # Arguments
    /// * `user`
    /// * `circle`
    /// * `status` - the new membership status, setting to "Inactive" will remove the membership
    /// * `disable_notifications`
    pub async fn set_user_circle_membership(
        &self,
        user: String,
        circle: String,
        status: MembershipStatus,
        disable_notifications: bool,
    ) -> Result<()> {
        // get current membership status
        let current = self
            .get_user_circle_membership(user.clone(), circle.clone())
            .await;

        if current == status {
            return Ok(());
        }

        let full_circle = match self.get_circle(circle.clone()).await {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // ...
        match status {
            MembershipStatus::Pending => {
                // add
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "INSERT INTO \"xcircle_memberships\" VALUES (?, ?, ?, ?)"
                    } else {
                        "INSERT INTO \"xcircle_memberships\" VALEUS ($1, $2, $3, $4)"
                    }
                    .to_string();

                let c = &self.base.db.client;
                if let Err(_) = sqlquery(&query)
                    .bind::<&String>(&user)
                    .bind::<&String>(&circle)
                    .bind::<&String>(&serde_json::to_string(&status).unwrap())
                    .bind::<&String>(&xsu_util::unix_epoch_timestamp().to_string())
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                // create notification
                if !disable_notifications {
                    if let Err(_) = self
                        .auth
                        .create_notification(NotificationCreate {
                            title: format!(
                                "[@{}](/@{}) has invited you to join their circle!",
                                full_circle.owner.username, full_circle.owner.username
                            ),
                            content: format!(
                                "{} has invited you to join \"{}\"",
                                full_circle.owner.username, full_circle.name
                            ),
                            address: format!("/circles/@{}/memberlist/accept", full_circle.name),
                            recipient: user,
                        })
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                };
            }
            MembershipStatus::Active => {
                // update
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "UPDATE \"xcircle_memberships\" SET \"membership\" = ? WHERE \"user\" = ? AND \"circle\" = ?"
                    } else {
                        "UPDATE \"xcircle_memberships\" SET (\"membership\") = (?) WHERE \"user\" = ? AND \"circle\" = ?"                    
                    }
                    .to_string();

                let c = &self.base.db.client;
                if let Err(_) = sqlquery(&query)
                    .bind::<&String>(&serde_json::to_string(&status).unwrap())
                    .bind::<&String>(&user)
                    .bind::<&String>(&circle)
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                self.base
                    .cachedb
                    .incr(format!(
                        "xsulib.sparkler.circle_memberships_count:{}",
                        circle
                    ))
                    .await;
            }
            MembershipStatus::Inactive => {
                // delete
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xcircle_memberships\" WHERE \"user\" = ? AND \"circle\" = ?"
                    } else {
                        "DELETE FROM \"xcircle_memberships\" WHERE \"user\" = ? AND \"circle\" = ?"
                    }
                    .to_string();

                let c = &self.base.db.client;
                if let Err(_) = sqlquery(&query)
                    .bind::<&String>(&user)
                    .bind::<&String>(&circle)
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                self.base
                    .cachedb
                    .decr(format!(
                        "xsulib.sparkler.circle_memberships_count:{}",
                        circle
                    ))
                    .await;
            }
        }

        // return
        Ok(())
    }

    /// Create a new circle
    ///
    /// Circles can only be created by non-anonymous users.
    ///
    /// # Arguments
    /// * `props` - [`CircleCreate`]
    /// * `owner` - the ID of the user creating the circle
    pub async fn create_circle(&self, props: CircleCreate, owner: String) -> Result<()> {
        let tag = Database::anonymous_tag(&owner);

        if tag.0 {
            // anonymous users cannot create circles
            return Err(DatabaseError::NotAllowed);
        }

        // make sure this name isn't taken
        if let Ok(_) = self.get_circle_by_name(props.name.clone()).await {
            return Err(DatabaseError::ValueError);
        }

        // check name length
        if props.name.len() > 32 {
            return Err(DatabaseError::ContentTooLong);
        }

        if props.name.len() < 2 {
            return Err(DatabaseError::ContentTooShort);
        }

        let blocked_names = &["new"];

        if blocked_names.contains(&props.name.as_str()) {
            return Err(DatabaseError::InvalidName);
        }

        // check characters used in name
        let regex = regex::RegexBuilder::new(r"[^\w_\-\.!]+$")
            .multi_line(true)
            .build()
            .unwrap();

        if regex.captures(&props.name).is_some() {
            return Err(DatabaseError::ValueError);
        }

        // check author permissions
        let owner = match self.auth.get_profile_by_username(owner.clone()).await {
            Ok(ua) => ua,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        if owner.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // ...
        let circle = Circle {
            name: props.name,
            id: utility::random_id(),
            owner,
            metadata: CircleMetadata { kv: HashMap::new() },
            timestamp: utility::unix_epoch_timestamp(),
        };

        // create circle
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xcircles\" VALUES (?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xcircles\" VALEUS ($1, $2, $3, $4, $5)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&circle.name)
            .bind::<&String>(&circle.id)
            .bind::<&String>(&circle.owner.id)
            .bind::<&String>(&serde_json::to_string(&circle.metadata).unwrap())
            .bind::<&String>(&circle.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // set membership

                // (send invite)
                if let Err(e) = self
                    .set_user_circle_membership(
                        circle.owner.id.clone(),
                        circle.id.clone(),
                        MembershipStatus::Pending,
                        true,
                    )
                    .await
                {
                    return Err(e);
                };

                // (accept invite)
                if let Err(e) = self
                    .set_user_circle_membership(
                        circle.owner.id,
                        circle.id,
                        MembershipStatus::Active,
                        true,
                    )
                    .await
                {
                    return Err(e);
                };

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Update an existing circle's `metadata`
    ///
    /// # Arguments
    /// * `id` - the ID of the circle
    /// * `metadata` - the new metadata
    /// * `user` - the user doing this
    pub async fn update_circle_metadata(
        &self,
        id: String,
        mut metadata: CircleMetadata,
        user: Profile,
    ) -> Result<()> {
        // make sure circle exists
        let circle = match self.get_circle(id.clone()).await {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // check permission
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != circle.owner.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check metadata kv
        let allowed_custom_keys = self.auth.allowed_custom_keys();

        for kv in metadata.kv.clone() {
            if !allowed_custom_keys.contains(&kv.0.as_str()) {
                metadata.kv.remove(&kv.0);
            }
        }

        // update circle
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xcircles\" SET \"metadata\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xcircles\" SET (\"metadata\") = ($1) WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&serde_json::to_string(&metadata).unwrap())
            .bind::<&String>(&circle.id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("xsulib.sparkler.circle:{}", id))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Delete an existing circle
    ///
    /// # Arguments
    /// * `id` - the ID of the circle
    /// * `user` - the user doing this
    pub async fn delete_circle(&self, id: String, user: Profile) -> Result<()> {
        // make sure circle exists
        let circle = match self.get_circle(id.clone()).await {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // check permission
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != circle.owner.id {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // update circle
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xcircles\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xcircles\" WHERE \"id\" = $2"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&circle.id)
            .execute(c)
            .await
        {
            Ok(_) => {
                // delete memberships
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xcircle_memberships\" WHERE \"circle\" = ?"
                    } else {
                        "DELETE FROM \"xcircle_memberships\" WHERE \"circle\" = $1"
                    }
                    .to_string();

                let c = &self.base.db.client;
                match sqlquery(&query)
                    .bind::<&String>(&circle.id)
                    .execute(c)
                    .await
                {
                    Ok(_) => {
                        // delete responses
                        let query: String = if (self.base.db.r#type == "sqlite")
                            | (self.base.db.r#type == "mysql")
                        {
                            "DELETE FROM \"xresponses\" WHERE \"author\" LIKE ?"
                        } else {
                            "DELETE FROM \"xresponses\" WHERE \"author\" LIKE $1"
                        }
                        .to_string();

                        let c = &self.base.db.client;
                        match sqlquery(&query)
                            .bind::<&String>(&format!(r"%{}", circle.id))
                            .execute(c)
                            .await
                        {
                            Ok(_) => {
                                // delete questions
                                let query: String = if (self.base.db.r#type == "sqlite")
                                    | (self.base.db.r#type == "mysql")
                                {
                                    "DELETE FROM \"xquestions\" WHERE \"recipient\" = ?"
                                } else {
                                    "DELETE FROM \"xquestions\" WHERE \"recipient\" = $1"
                                }
                                .to_string();

                                let c = &self.base.db.client;
                                match sqlquery(&query)
                                    .bind::<&String>(&format!(r"@{}", circle.id))
                                    .execute(c)
                                    .await
                                {
                                    Ok(_) => {
                                        self.base
                                            .cachedb
                                            .remove(format!("xsulib.sparkler.circle:{}", id))
                                            .await;

                                        self.base
                                            .cachedb
                                            .remove(format!(
                                                "xsulib.sparkler.circle_memberships_count:{}",
                                                id
                                            ))
                                            .await;

                                        // return
                                        return Ok(());
                                    }
                                    Err(_) => return Err(DatabaseError::Other),
                                }
                            }
                            Err(_) => return Err(DatabaseError::Other),
                        }
                    }
                    Err(_) => return Err(DatabaseError::Other),
                }
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }

    /// Get all responses by their circle
    ///
    /// ## Arguments:
    /// * `circle`
    pub async fn get_responses_by_circle(
        &self,
        circle: String,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // get circle
        let circle = match self.get_circle(circle.clone()).await {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // build member list
        let members = self
            .get_circle_memberships(circle.id.clone())
            .await
            .unwrap();
        let mut query_string = String::new();

        for member in members {
            query_string.push_str(&format!(" OR \"author\" = '{}%{}'", member.id, circle.id));
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"author\" = ?{query_string} ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"author\" = $1{query_string} ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&circle.id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        Ok(res)
    }

    /// Get all responses by their circle, 50 at a time
    ///
    /// ## Arguments:
    /// * `circle`
    pub async fn get_responses_by_circle_paginated(
        &self,
        circle: String,
        page: i32,
    ) -> Result<Vec<(Question, QuestionResponse, usize, usize)>> {
        // get circle
        let circle = match self.get_circle(circle.clone()).await {
            Ok(c) => c,
            Err(e) => return Err(e),
        };

        // build member list
        let members = self
            .get_circle_memberships(circle.id.clone())
            .await
            .unwrap();
        let mut query_string = String::new();

        for member in members {
            query_string.push_str(&format!(" OR \"author\" = '{}%{}'", member.id, circle.id));
        }

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = ?{query_string} ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        } else {
            format!("SELECT * FROM \"xresponses\" WHERE \"author\" = $1{query_string} ORDER BY \"timestamp\" DESC LIMIT 50 OFFSET {}", page * 50)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&circle.id.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(Question, QuestionResponse, usize, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_response(res).await {
                        Ok(r) => r,
                        Err(e) => return Err(e),
                    });
                }

                out
            }
            Err(_) => return Err(DatabaseError::Other),
        };

        // return
        Ok(res)
    }

    /// Get the number of responses by their circle ID
    ///
    /// ## Arguments:
    /// * `circle`
    pub async fn get_response_count_by_circle(&self, circle: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.response_count:{}", circle))
            .await
        {
            return count.parse::<usize>().unwrap_or(0);
        };

        // fetch from database
        let count = self
            .get_responses_by_circle(circle.clone())
            .await
            .unwrap_or(Vec::new())
            .len();

        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.response_count:{}", circle),
                count.to_string(),
            )
            .await;

        count
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
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
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
        let relationship = self.get_user_relationship(one.clone(), two.clone()).await;

        if relationship.0 == status {
            return Ok(());
        }

        let uone = match self.get_profile(relationship.1).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        let utwo = match self.get_profile(relationship.2).await {
            Ok(ua) => ua,
            Err(e) => return Err(e),
        };

        // ...
        match status {
            RelationshipStatus::Blocked => {
                if relationship.0 != RelationshipStatus::Unknown {
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
                        .bind::<&String>(&xsu_util::unix_epoch_timestamp().to_string())
                        .execute(c)
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }
            }
            RelationshipStatus::Pending => {
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
                    .bind::<&String>(&xsu_util::unix_epoch_timestamp().to_string())
                    .execute(c)
                    .await
                {
                    return Err(DatabaseError::Other);
                };

                // create notification
                if !disable_notifications {
                    if let Err(_) = self
                        .auth
                        .create_notification(NotificationCreate {
                            title: format!(
                                "[@{}](/@{}) has sent you a friend request!",
                                uone.username, uone.username
                            ),
                            content: format!("{} wants to be your friend.", uone.username),
                            address: format!("/@{}/relationship/friend_accept", uone.username),
                            recipient: utwo.id,
                        })
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
                    .incr(format!("xsulib.sparkler.friends_count:{}", one))
                    .await;

                self.base
                    .cachedb
                    .incr(format!("xsulib.sparkler.friends_count:{}", two))
                    .await;

                // create notification
                if !disable_notifications {
                    if let Err(_) = self
                        .auth
                        .create_notification(NotificationCreate {
                            title: "Your friend request has been accepted!".to_string(),
                            content: format!("{} has accepted your friend request.", utwo.username),
                            address: String::new(),
                            recipient: uone.id,
                        })
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

                self.base
                    .cachedb
                    .decr(format!("xsulib.sparkler.friends_count:{}", one))
                    .await;

                self.base
                    .cachedb
                    .decr(format!("xsulib.sparkler.friends_count:{}", two))
                    .await;
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
    ) -> Result<Vec<(Profile, RelationshipStatus)>> {
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
                    let res = self.base.textify_row(row, Vec::new()).0;

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
}
