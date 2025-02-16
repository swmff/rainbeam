use langbeam::LangFile;
use pathbufd::PathBufD;
use serde::{Serialize, Deserialize};
use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use crate::model::{Profile, RelationshipStatus};
use reva_axum::Template;
use rainbeam_shared::config::Config;

pub static LAYOUTS: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Template)]
#[template(path = "profile/layout_components/renderer.html")]
pub struct RendererTemplate<'a> {
    pub other: &'a Profile,
    pub component: &'a LayoutComponent,
    // profile
    pub config: &'a Config,
    pub profile: &'a Option<Box<Profile>>,
    pub lang: &'a LangFile,
    pub response_count: usize,
    pub questions_count: usize,
    pub followers_count: usize,
    pub following_count: usize,
    pub friends_count: usize,
    pub is_following: bool,
    pub is_following_you: bool,
    pub relationship: RelationshipStatus,
    pub lock_profile: bool,
    pub disallow_anonymous: bool,
    pub require_account: bool,
    pub hide_social: bool,
    pub is_powerful: bool, // at least "manager"
    pub is_helper: bool,   // at least "helper"
    pub is_self: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ComponentName {
    /// An empty element.
    #[serde(alias = "empty")]
    Empty,
    /// A flex container.
    #[serde(alias = "flex")]
    Flex,
    /// The profile's banner.
    #[serde(alias = "banner")]
    Banner,
    /// A markdown block.
    #[serde(alias = "markdown")]
    Markdown,
    /// The profile's feed (responses, questions, etc.).
    #[serde(alias = "feed")]
    Feed,
    /// The profile's `profile_card` (avatar, biography, sidebar, links, actions).
    #[serde(alias = "box")]
    Box,
    /// The profile's tabs (social, feed/questions/mod, etc.).
    #[serde(alias = "tabs")]
    Tabs,
    /// The profile's ask box.
    #[serde(alias = "ask")]
    Ask,
}

impl Default for ComponentName {
    fn default() -> Self {
        Self::Empty
    }
}

/// A component of the layout. Essentially just a limited description of an HTML element.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LayoutComponent {
    #[serde(default)]
    pub json: String,
    #[serde(default)]
    pub component: ComponentName,
    #[serde(default)]
    pub options: HashMap<String, String>,
    #[serde(default)]
    pub children: Vec<LayoutComponent>,
}

impl Default for LayoutComponent {
    fn default() -> Self {
        Self {
            json: String::new(),
            component: ComponentName::Empty,
            options: HashMap::new(),
            children: Vec::new(),
        }
    }
}

impl LayoutComponent {
    /// Get the value of an option in the `options` map. Accepts a default substitute.
    pub fn option(&self, k: &str, d: Option<String>) -> String {
        match self.options.get(k) {
            Some(v) => v.to_owned(),
            None => {
                if let Some(d) = d {
                    d
                } else {
                    String::new()
                }
            }
        }
    }

    /// Render the component as HTML. Since this is a profile layout, we require
    /// a reference to the [`Profile`] this layout is being rendered for.
    pub fn render(
        &self,
        user: &Profile,
        // this is absurd
        config: &Config,
        profile: &Option<Box<Profile>>,
        lang: &LangFile,
        response_count: usize,
        questions_count: usize,
        followers_count: usize,
        following_count: usize,
        friends_count: usize,
        is_following: bool,
        is_following_you: bool,
        relationship: RelationshipStatus,
        lock_profile: bool,
        disallow_anonymous: bool,
        require_account: bool,
        hide_social: bool,
        is_powerful: bool,
        is_helper: bool,
        is_self: bool,
    ) -> String {
        use ComponentName as T;

        // json import
        // all imports are relative to `.config/layouts/default.json`
        if !self.json.is_empty() {
            let reader = match LAYOUTS.read() {
                Ok(r) => r,
                Err(_) => {
                    LAYOUTS.clear_poison();
                    return String::new();
                }
            };

            return {
                if let Some(l) = (*reader).get(&self.json) {
                    serde_json::from_str::<LayoutComponent>(l)
                } else {
                    let l = match std::fs::read_to_string(PathBufD::current().extend(&[
                        ".config",
                        "layouts",
                        self.json.as_str(),
                    ])) {
                        Ok(l) => l,
                        Err(_) => return String::new(),
                    };

                    drop(reader); // drop the reader so we can create a writer
                    let mut writer = LAYOUTS.write().unwrap();
                    (*writer).insert(self.json.clone(), l.clone());

                    serde_json::from_str::<LayoutComponent>(&l)
                }
                .unwrap()
                .render(
                    user,
                    config,
                    profile,
                    lang,
                    response_count,
                    questions_count,
                    followers_count,
                    following_count,
                    friends_count,
                    is_following,
                    is_following_you,
                    relationship,
                    lock_profile,
                    disallow_anonymous,
                    require_account,
                    hide_social,
                    is_powerful,
                    is_helper,
                    is_self,
                )
            };
        }

        // regular
        match self.component {
            T::Flex => format!(
                "<div class=\"flex {} {} {} {}\">{}</div>",
                // extra classes
                {
                    let direction = self.option("direction", None);
                    if !direction.is_empty() {
                        format!("flex-{direction}")
                    } else {
                        String::new()
                    }
                },
                {
                    let gap = self.option("gap", None);
                    if !gap.is_empty() {
                        format!("gap-{gap}")
                    } else {
                        String::new()
                    }
                },
                {
                    let collapse = self.option("collapse", None);
                    if !collapse.is_empty() {
                        "flex-collapse"
                    } else {
                        ""
                    }
                },
                {
                    let width = self.option("width", None);
                    if !width.is_empty() {
                        format!("w-{width}")
                    } else {
                        String::new()
                    }
                },
                // children
                {
                    let mut children: String = String::new();

                    for child in &self.children {
                        children.push_str(
                            &RendererTemplate {
                                other: user,
                                component: child,
                                // profile
                                config,
                                profile,
                                lang,
                                response_count,
                                questions_count,
                                followers_count,
                                following_count,
                                friends_count,
                                is_following,
                                is_following_you,
                                relationship: relationship.clone(),
                                lock_profile,
                                disallow_anonymous,
                                require_account,
                                hide_social,
                                is_powerful,
                                is_helper,
                                is_self,
                            }
                            .render()
                            .unwrap(),
                        );
                    }

                    children
                }
            ),
            T::Markdown => format!(
                "<div class=\"card\">{}</div>",
                rainbeam_shared::ui::render_markdown(&self.option("text", None))
            ),
            _ => format!("ComponentName::{:?}", self.component),
        }
    }
}

impl AsRef<LayoutComponent> for LayoutComponent {
    fn as_ref(&self) -> &LayoutComponent {
        // the fact that this is a fix is crazy
        self
    }
}
