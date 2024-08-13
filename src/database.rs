use crate::config::Config;
use crate::model::{CommentCreate, QuestionCreate, QuestionResponse, ResponseComment, ResponseCreate};
use crate::model::{DatabaseError, Question};

use xsu_dataman::utility;
use xsu_dataman::query as sqlquery;
use xsu_authman::model::{NotificationCreate, Permission, Profile};

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

    // ...

    // questions

    /// Get an existing question
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_question(&self, id: String) -> Result<Question> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.question:{}", id))
            .await
        {
            Some(c) => return Ok(serde_json::from_str::<Question>(c.as_str()).unwrap()),
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
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let question = Question {
            author: res.get("author").unwrap().to_string(),
            recipient: res.get("recipient").unwrap().to_string(),
            content: res.get("content").unwrap().to_string(),
            id: res.get("id").unwrap().to_string(),
            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("xsulib.sparkler.question:{}", id),
                serde_json::to_string::<Question>(&question).unwrap(),
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
                        author: res.get("author").unwrap().to_string(),
                        recipient: res.get("recipient").unwrap().to_string(),
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
                            author: res.get("author").unwrap().to_string(),
                            recipient: res.get("recipient").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: res.get("id").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.base
                            .cachedb
                            .get(format!("xsulib.sparker.question_response_count:{}", id))
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
    ) -> Result<Vec<(Question, i32)>> {
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
                let mut out: Vec<(Question, i32)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();
                    out.push((
                        Question {
                            author: res.get("author").unwrap().to_string(),
                            recipient: res.get("recipient").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: res.get("id").unwrap().to_string(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.base
                            .cachedb
                            .get(format!("xsulib.sparker.question_response_count:{}", id))
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
            query_string.push_str(&format!(" OR \"author\" = '{}'", follow.following));
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
            .bind::<&String>(&user.username.to_lowercase())
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
                            author: res.get("author").unwrap().to_string(),
                            recipient: res.get("recipient").unwrap().to_string(),
                            content: res.get("content").unwrap().to_string(),
                            id: id.clone(),
                            timestamp: res.get("timestamp").unwrap().parse::<u128>().unwrap(),
                        },
                        // get the number of responses the question has
                        self.base
                            .cachedb
                            .get(format!("xsulib.sparker.question_response_count:{}", id))
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

    /// Get the number of global questions by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_global_questions_count_by_author(&self, author: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparker.global_questions_count:{}", author))
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
                format!("xsulib.sparker.global_question_count:{}", author),
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
        if (props.content.trim().len() < 2) | (props.content.len() > 250) {
            return Err(DatabaseError::ValueError);
        }

        // check recipient
        // "@" is the recipient we use for global questions (questions anybody can respond to)
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

            if (block_anonymous == true) && author == "anonymous" {
                return Err(DatabaseError::NotAllowed);
            }

            if block_list.contains(&format!("<@{author}>")) {
                return Err(DatabaseError::NotAllowed);
            }
        } else {
            // anonymous users cannot ask global questions
            if author == "anonymous" {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // check author permissions
        if author != "anonymous" {
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

        // ...
        let question = Question {
            author,
            recipient: props.recipient,
            content: props.content,
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
            .bind::<&String>(&question.author)
            .bind::<&String>(&question.recipient)
            .bind::<&String>(&question.content)
            .bind::<&String>(&question.id)
            .bind::<&String>(&question.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // incr questions count
                if question.recipient == "@" {
                    self.base
                        .cachedb
                        .incr(format!(
                            "xsulib.sparker.global_question_count:{}",
                            question.author
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
        if (user.username != question.recipient) && (user.username != question.author) {
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
                if question.recipient == "@" {
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
                            "xsulib.sparker.question_response_count:{}",
                            question.id
                        ))
                        .await;

                    // decr questions count
                    self.base
                        .cachedb
                        .decr(format!(
                            "xsulib.sparker.global_question_count:{}",
                            question.author
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
    pub async fn get_response(&self, id: String) -> Result<(QuestionResponse, usize)> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("xsulib.sparkler.response:{}", id))
            .await
        {
            Some(c) => {
                match serde_json::from_str::<(QuestionResponse, usize)>(c.as_str()) {
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
            author: res.get("author").unwrap().to_string(),
            question: match serde_json::from_str(res.get("question").unwrap()) {
                Ok(q) => q,
                Err(_) => return Err(DatabaseError::ValueError),
            },
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
        Ok((response, self.get_comment_count_by_response(id).await))
    }

    /// Get all responses by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_responses_by_author(
        &self,
        author: String,
    ) -> Result<Vec<(QuestionResponse, usize)>> {
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
                let mut out: Vec<(QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        QuestionResponse {
                            author: res.get("author").unwrap().to_string(),
                            question: match serde_json::from_str(res.get("question").unwrap()) {
                                Ok(q) => q,
                                Err(_) => return Err(DatabaseError::ValueError),
                            },
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
    ) -> Result<Vec<(QuestionResponse, usize)>> {
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
                let mut out: Vec<(QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        QuestionResponse {
                            author: res.get("author").unwrap().to_string(),
                            question: match serde_json::from_str(res.get("question").unwrap()) {
                                Ok(q) => q,
                                Err(_) => return Err(DatabaseError::ValueError),
                            },
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

    /// Get the number of responses by their author
    ///
    /// ## Arguments:
    /// * `author`
    pub async fn get_response_count_by_author(&self, author: String) -> usize {
        // attempt to fetch from cache
        if let Some(count) = self
            .base
            .cachedb
            .get(format!("xsulib.sparker.response_count:{}", author))
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
                format!("xsulib.sparker.response_count:{}", author),
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
    ) -> Result<Vec<(QuestionResponse, usize)>> {
        // get following
        let following = match self.auth.get_following(user.clone()).await {
            Ok(f) => f,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // check user permissions
        // returning NotAllowed here will block them from viewing their timeline
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
            query_string.push_str(&format!(" OR \"author\" = '{}'", follow.following));
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
            .bind::<&String>(&user.username.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        QuestionResponse {
                            author: res.get("author").unwrap().to_string(),
                            question: match serde_json::from_str(res.get("question").unwrap()) {
                                Ok(q) => q,
                                Err(_) => return Err(DatabaseError::ValueError),
                            },
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

    /// Get all responses by their question ID
    ///
    /// ## Arguments:
    /// * `id`
    pub async fn get_responses_by_question(
        &self,
        id: String,
    ) -> Result<Vec<(QuestionResponse, usize)>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xresponses\" WHERE \"question\" LIKE ? ORDER BY \"timestamp\" DESC"
        } else {
            "SELECT * FROM \"xresponses\" WHERE \"question\" LIKE $1 ORDER BY \"timestamp\" DESC"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&format!("%\"id\":\"{}\"%", id))
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<(QuestionResponse, usize)> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    let id = res.get("id").unwrap().to_string();

                    out.push((
                        QuestionResponse {
                            author: res.get("author").unwrap().to_string(),
                            question: match serde_json::from_str(res.get("question").unwrap()) {
                                Ok(q) => q,
                                Err(_) => return Err(DatabaseError::ValueError),
                            },
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
        if question.recipient != "@" {
            if question.recipient != author {
                // cannot respond to a question not asked to us
                return Err(DatabaseError::NotAllowed);
            }
        } else {
            // TODO: check author privacy settings
        }

        // check content length
        if props.content.len() > 500 {
            return Err(DatabaseError::ValueError);
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

        // ...
        let response = QuestionResponse {
            author: author.username,
            question: question.clone(),
            content: props.content,
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
            .bind::<&String>(&response.author)
            .bind::<&String>(&match serde_json::to_string(&response.question) {
                Ok(s) => s,
                Err(_) => return Err(DatabaseError::ValueError),
            })
            .bind::<&String>(&response.content)
            .bind::<&String>(&response.id)
            .bind::<&String>(&response.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification
                if (question.recipient != question.author) && question.author != "anonymous" {
                    if let Err(_) = self
                        .auth
                        .create_notification(NotificationCreate {
                            title: format!(
                                "[@{}](/@{}) responded to a question you asked!",
                                response.author, response.author
                            ),
                            content: format!(
                                "{}: \"{}...\"",
                                response.author,
                                // we're only going to show 50 characters of the response in the notification
                                response
                                    .content
                                    .clone()
                                    .chars()
                                    .take(50)
                                    .collect::<String>()
                            ),
                            address: format!("/response/{}", response.id),
                            recipient: question.author,
                        })
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }

                // handle global questions
                if question.recipient == "@" {
                    // this is a global ask, we need to respond to it and then just move on

                    // bump question response count
                    self.base
                        .cachedb
                        .incr(format!(
                            "xsulib.sparker.question_response_count:{}",
                            question.id
                        ))
                        .await;

                    // bump response count
                    self.base
                        .cachedb
                        .incr(format!("xsulib.sparker.response_count:{}", response.author))
                        .await;

                    return Ok(());
                }

                // ...

                // delete question
                let query: String =
                    if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql") {
                        "DELETE FROM \"xquestions\" WHERE \"id\" = ?"
                    } else {
                        "DELETE FROM \"xquestions\" WHERE \"id\" = $1"
                    }
                    .to_string();

                let c = &self.base.db.client;
                match sqlquery(&query)
                    .bind::<&String>(&props.question)
                    .execute(c)
                    .await
                {
                    Ok(_) => {
                        // remove from cache
                        self.base
                            .cachedb
                            .remove(format!("xsulib.sparkler.question:{}", props.question))
                            .await;

                        // bump response count
                        self.base
                            .cachedb
                            .incr(format!("xsulib.sparker.response_count:{}", response.author))
                            .await;

                        // return
                        return Ok(());
                    }
                    Err(_) => return Err(DatabaseError::Other),
                };
            }
            Err(_) => return Err(DatabaseError::Other),
        };
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
            Ok(q) => q.0,
            Err(e) => return Err(e),
        };

        // check username
        if user.username != response.author {
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
                    .decr(format!("xsulib.sparker.response_count:{}", response.author))
                    .await;

                // decr global question response count
                if response.question.recipient == "@" {
                    self.base
                        .cachedb
                        .decr(format!(
                            "xsulib.sparker.question_response_count:{}",
                            response.question.id
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
            author: res.get("author").unwrap().to_string(),
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
                        author: res.get("author").unwrap().to_string(),
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
                        author: res.get("author").unwrap().to_string(),
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
            .get(format!("xsulib.sparker.comment_count:{}", id))
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
                format!("xsulib.sparker.comment_count:{}", id),
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
            Ok(q) => q.0,
            Err(e) => return Err(e),
        };

        if author == "anonymous" {
            return Err(DatabaseError::NotAllowed);
        }

        // check content length
        if props.content.len() > 250 {
            return Err(DatabaseError::ValueError);
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

        // ...
        let comment = ResponseComment {
            author: author.username,
            response: response.id.clone(),
            content: props.content,
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
            .bind::<&String>(&comment.author)
            .bind::<&String>(&comment.response)
            .bind::<&String>(&comment.content)
            .bind::<&String>(&comment.id)
            .bind::<&String>(&comment.timestamp.to_string())
            .execute(c)
            .await
        {
            Ok(_) => {
                // create notification
                if response.author != comment.author {
                    if let Err(_) = self
                        .auth
                        .create_notification(NotificationCreate {
                            title: format!(
                                "[@{}](/@{}) commented on a response you created!",
                                comment.author, comment.author
                            ),
                            content: format!(
                                "{}: \"{}...\"",
                                comment.author,
                                // we're only going to show 50 characters of the response in the notification
                                comment.content.clone().chars().take(50).collect::<String>()
                            ),
                            address: format!("/response/{}", response.id),
                            recipient: response.author,
                        })
                        .await
                    {
                        return Err(DatabaseError::Other);
                    };
                }

                // bump comment count
                self.base
                    .cachedb
                    .incr(format!("xsulib.sparker.comment_count:{}", response.id))
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
        if user.username != comment.author {
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
                    .decr(format!("xsulib.sparker.comment_count:{}", comment.response))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
    }
}
