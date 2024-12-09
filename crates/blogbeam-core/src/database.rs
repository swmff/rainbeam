use std::collections::HashMap;

use databeam::{utility, query as sqlquery, DefaultReturn};
use authbeam::model::{Profile, Permission};
use rainbeam::config::Config;
use crate::model::{DatabaseError, Post, PostContext, PostCreate, PostEdit};

pub type Result<T> = std::result::Result<T, DatabaseError>;

#[derive(Clone)]
pub struct Database {
    pub base: databeam::StarterDatabase,
    pub auth: authbeam::Database,
    pub server_options: Config,
    langs: HashMap<String, langbeam::LangFile>,
}

impl Database {
    pub async fn new(
        opts: databeam::DatabaseOpts,
        auth: authbeam::Database,
        server_options: Config,
    ) -> Self {
        Self {
            base: databeam::StarterDatabase::new(opts).await,
            auth,
            server_options,
            langs: langbeam::read_langs(),
        }
    }

    /// Init database
    pub async fn init(&self) {
        // create tables
        let c = &self.base.db.client;

        // create posts table
        let _ = sqlquery(
            "CREATE TABLE IF NOT EXISTS \"xposts\" (
                id        TEXT,
                slug      TEXT,
                owner     TEXT,
                published TEXT,
                edited    TEXT,
                content   TEXT,
                context   TEXT
            )",
        )
        .execute(c)
        .await;
    }

    // language

    /// Get a [`LangFile`] given its ID
    ///
    /// Returns `net.rainbeam.langs:en-US` if the given file cannot be found.
    pub fn lang(&self, id: &str) -> langbeam::LangFile {
        if id.is_empty() {
            // don't even try to fetch an empty id
            return self
                .langs
                .get("net.rainbeam.langs:en-US")
                .unwrap()
                .to_owned();
        } else if (id == "aa-BB") | (id == "net.rainbeam.langs.testing:aa-BB") {
            // debug
            return langbeam::LangFile::default();
        }

        self.langs
            .get(id)
            .unwrap_or(self.langs.get("net.rainbeam.langs:en-US").unwrap())
            .to_owned()
    }

    // posts

    /// Get a [`Post`] from a database result
    pub async fn gimme_post(&self, res: HashMap<String, String>) -> Result<Post> {
        Ok(Post {
            id: res.get("id").unwrap().to_string(),
            slug: res.get("slug").unwrap().to_string(),
            owner: res.get("owner").unwrap().to_string(),
            published: res.get("published").unwrap().parse::<u128>().unwrap(),
            edited: res.get("edited").unwrap().parse::<u128>().unwrap(),
            content: res.get("content").unwrap().to_string(),
            context: match serde_json::from_str(res.get("context").unwrap()) {
                Ok(ctx) => ctx,
                Err(_) => return Err(DatabaseError::ValueError),
            },
        })
    }

    /// Get an existing site (given its id)
    ///
    /// # Arguments
    /// * `id`
    pub async fn get_post(&self, id: String) -> Result<Post> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.app.post:{}", id))
            .await
        {
            Some(c) => {
                match serde_json::from_str::<HashMap<String, String>>(c.as_str()) {
                    Ok(res) => {
                        return Ok(match self.gimme_post(res).await {
                            Ok(r) => r,
                            Err(e) => return Err(e),
                        })
                    }
                    Err(_) => {
                        // we're storing a bad version that couldn't deserialize, we don't need that...
                        self.base
                            .cachedb
                            .remove(format!("rbeam.app.post:{}", id))
                            .await
                    }
                };
            }
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xposts\" WHERE \"id\" LIKE ?"
        } else {
            "SELECT * FROM \"xposts\" WHERE \"id\" LIKE $1"
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
        let site = match self.gimme_post(res).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.app.post:{}", id),
                serde_json::to_string::<Post>(&site).unwrap(),
            )
            .await;

        // return
        Ok(site)
    }

    /// Get an existing post (given its slug)
    ///
    /// # Arguments
    /// * `slug`
    pub async fn get_post_by_slug(&self, slug: String) -> Result<Post> {
        // check in cache
        match self
            .base
            .cachedb
            .get(format!("rbeam.app.post/slug:{}", slug))
            .await
        {
            Some(c) => {
                match serde_json::from_str::<HashMap<String, String>>(c.as_str()) {
                    Ok(res) => {
                        return Ok(match self.gimme_post(res).await {
                            Ok(r) => r,
                            Err(e) => return Err(e),
                        })
                    }
                    Err(_) => {
                        // we're storing a bad version that couldn't deserialize, we don't need that...
                        self.base
                            .cachedb
                            .remove(format!("rbeam.app.post/slug:{}", slug))
                            .await
                    }
                };
            }
            None => (),
        };

        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "SELECT * FROM \"xposts\" WHERE \"slug\" = ?"
        } else {
            "SELECT * FROM \"xposts\" WHERE \"slug\" = ?"
        }
        .to_string();

        let c = &self.base.db.client;
        let res = match sqlquery(&query).bind::<&String>(&slug).fetch_one(c).await {
            Ok(p) => self.base.textify_row(p, Vec::new()).0,
            Err(_) => return Err(DatabaseError::NotFound),
        };

        // return
        let site = match self.gimme_post(res).await {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        // store in cache
        self.base
            .cachedb
            .set(
                format!("rbeam.app.post/slug:{}", slug),
                serde_json::to_string::<Post>(&site).unwrap(),
            )
            .await;

        // return
        Ok(site)
    }

    /// Get all pages by their author, 25 at a time
    ///
    /// # Arguments
    /// * `author`
    pub async fn get_posts_by_author_paginated(
        &self,
        author: String,
        page: i32,
    ) -> Result<Vec<Post>> {
        // pull from database
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            format!("SELECT * FROM \"xposts\" WHERE \"owner\" = ? ORDER BY \"edited\" DESC LIMIT 25 OFFSET {}", page * 25)
        } else {
            format!("SELECT * FROM \"xposts\" WHERE \"owner\" = $1 ORDER BY \"edited\" DESC LIMIT 25 OFFSET {}", page * 25)
        };

        let c = &self.base.db.client;
        let res = match sqlquery(&query)
            .bind::<&String>(&author.to_lowercase())
            .fetch_all(c)
            .await
        {
            Ok(p) => {
                let mut out: Vec<Post> = Vec::new();

                for row in p {
                    let res = self.base.textify_row(row, Vec::new()).0;
                    out.push(match self.gimme_post(res).await {
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

    /// Create a new post
    ///
    /// # Arguments
    /// * `props` - [`PostCreate`]
    /// * `author` - the ID of the user creating the post
    pub async fn create_post(&self, props: PostCreate, author: String) -> Result<Post> {
        // make sure site doesn't already exist
        if let Ok(_) = self.get_post_by_slug(props.slug.clone()).await {
            return Err(DatabaseError::InvalidNameUnique);
        }

        // check author permissions
        let author = match self.auth.get_profile(author.clone()).await {
            Ok(ua) => ua,
            Err(_) => return Err(DatabaseError::Other),
        };

        if author.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        // check slug length
        if props.slug.len() < 2 {
            return Err(DatabaseError::ContentTooLong);
        }

        if props.slug.len() > 32 {
            return Err(DatabaseError::ContentTooLong);
        }

        // ...
        let site = Post {
            id: utility::random_id(),
            slug: props.slug,
            owner: author.id,
            published: utility::unix_epoch_timestamp(),
            edited: utility::unix_epoch_timestamp(),
            content: props.content,
            context: PostContext {},
        };

        // create page
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "INSERT INTO \"xposts\" VALUES (?, ?, ?, ?, ?, ?, ?)"
        } else {
            "INSERT INTO \"xposts\" VALEUS ($1, $2, $3, $4, $5, $6, $7)"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&site.id)
            .bind::<&String>(&site.slug)
            .bind::<&String>(&site.owner)
            .bind::<&String>(&site.published.to_string())
            .bind::<&String>(&site.edited.to_string())
            .bind::<&String>(&site.content)
            .bind::<&String>(&match serde_json::to_string(&site.context) {
                Ok(s) => s,
                Err(_) => return Err(DatabaseError::ValueError),
            })
            .execute(c)
            .await
        {
            Ok(_) => Ok(site),
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Update an existing post's content
    ///
    /// # Arguments
    /// * `id`
    /// * `content`
    /// * `user` - the user doing this
    pub async fn update_post_content(
        &self,
        id: String,
        props: PostEdit,
        user: Box<Profile>,
    ) -> Result<()> {
        // make sure the post exists
        let post = match self.get_post(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != post.owner {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // update post
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "UPDATE \"xposts\" SET \"content\" = ?, \"slug\" = ?, \"edited\" = ? WHERE \"id\" = ?"
        } else {
            "UPDATE \"xposts\" SET (\"content\", \"slug\", \"edited\") = ($1, $2, $3) WHERE \"id\" = $4"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query)
            .bind::<&String>(&props.content)
            .bind::<&String>(&if !props.new_slug.is_empty() {
                props.new_slug
            } else {
                post.slug.clone()
            })
            .bind::<&String>(&utility::unix_epoch_timestamp().to_string())
            .bind::<&String>(&id)
            .execute(c)
            .await
        {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.app.post:{id}"))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.app.post:{}:{}", post.owner, post.slug))
                    .await;

                Ok(())
            }
            Err(_) => Err(DatabaseError::Other),
        }
    }

    /// Delete an existing post
    ///
    /// Page can only be deleted by their owner.
    ///
    /// # Arguments
    /// * `id` - the ID of the page
    /// * `user` - the user doing this
    pub async fn delete_post(&self, id: String, user: Box<Profile>) -> Result<()> {
        // make sure page exists
        let site = match self.get_post(id.clone()).await {
            Ok(q) => q,
            Err(e) => return Err(e),
        };

        // check user permissions
        if user.group == -1 {
            // group -1 (even if it exists) is for marking users as banned
            return Err(DatabaseError::NotAllowed);
        }

        if user.id != site.owner {
            // check permission
            let group = match self.auth.get_group_by_id(user.group).await {
                Ok(g) => g,
                Err(_) => return Err(DatabaseError::Other),
            };

            if !group.permissions.contains(&Permission::Manager) {
                return Err(DatabaseError::NotAllowed);
            }
        }

        // delete post
        let query: String = if (self.base.db.r#type == "sqlite") | (self.base.db.r#type == "mysql")
        {
            "DELETE FROM \"xposts\" WHERE \"id\" = ?"
        } else {
            "DELETE FROM \"xposts\" WHERE \"id\" = $1"
        }
        .to_string();

        let c = &self.base.db.client;
        match sqlquery(&query).bind::<&String>(&id).execute(c).await {
            Ok(_) => {
                self.base
                    .cachedb
                    .remove(format!("rbeam.app.post:{id}"))
                    .await;

                self.base
                    .cachedb
                    .remove(format!("rbeam.app.post:{}:{}", site.owner, site.slug))
                    .await;

                // return
                return Ok(());
            }
            Err(_) => return Err(DatabaseError::Other),
        };
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
