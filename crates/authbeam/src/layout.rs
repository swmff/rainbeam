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

/// Renderer which does not require a bunch of junk to render.
///
/// Does not render profile-specific components properly. They will be replaced with
/// their name.
#[derive(Template)]
#[template(path = "profile/layout_components/free_renderer.html")]
pub struct FreeRendererTemplate<'a> {
    pub other: &'a Profile,
    pub component: &'a LayoutComponent,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
    /// The profile's tabs (social, feed/questions/mod, etc.).
    #[serde(alias = "tabs")]
    Tabs,
    /// The profile's ask box.
    #[serde(alias = "ask")]
    Ask,
    /// The profile's name and avatar.
    #[serde(alias = "name")]
    Name,
    /// The profile's about section (about and biography).
    #[serde(alias = "about")]
    About,
    /// The profile's action buttons.
    #[serde(alias = "actions")]
    Actions,
    /// A `<hr>` element.
    #[serde(alias = "divider")]
    Divider,
    /// A `<style>` element.
    #[serde(alias = "style")]
    Style,
    /// The site footer.
    #[serde(alias = "footer")]
    Footer,
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
    /// Create a [`LayoutComponent`] from the name of a file in `./.config/layouts`.
    pub fn from_json_file(file: &str) -> Self {
        Self {
            json: file.to_string(),
            component: ComponentName::Empty,
            options: HashMap::new(),
            children: Vec::new(),
        }
        .fill()
    }

    /// Follow component template to get full template.
    ///
    /// All imports are relative to `./.config/layouts`.
    pub fn fill(&self) -> LayoutComponent {
        if !self.json.is_empty() {
            let reader = match LAYOUTS.read() {
                Ok(r) => r,
                Err(_) => {
                    LAYOUTS.clear_poison();
                    return LayoutComponent::default();
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
                        Err(_) => return LayoutComponent::default(),
                    };

                    drop(reader); // drop the reader so we can create a writer
                    let mut writer = LAYOUTS.write().unwrap();
                    (*writer).insert(self.json.clone(), l.clone());

                    serde_json::from_str::<LayoutComponent>(&l)
                }
                .unwrap()
            };
        }

        self.to_owned()
    }

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

    /// Render the component as HTML. Skips rendering with extra junk.
    ///
    /// See [`FreeRendererTemplate`].
    /// See [`LayoutComponent::render_with_junk`] to include junk.
    pub fn render(&self, user: &Profile) -> String {
        use ComponentName as T;

        // json import
        if !self.json.is_empty() {
            return self.fill().render(user);
        }

        // regular
        match self.component {
            T::Flex => format!(
                "<div class=\"flex {} {} {} {} {}\" style=\"{}\" id=\"{}\">{}</div>",
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
                self.option("class", None),
                self.option("style", None),
                self.option("id", None),
                // children
                {
                    let mut children: String = String::new();

                    for child in &self.children {
                        children.push_str(&child.render(user));
                    }

                    children
                }
            ),
            T::Divider => format!("<hr class=\"{}\" />", self.option("class", None)),
            T::Markdown => format!(
                "<div class=\"{}\">{}</div>",
                self.option("class", None),
                rainbeam_shared::ui::render_markdown(&self.option("text", None))
            ),
            T::Style => format!(
                "<style>{}</style>",
                self.option("data", None).replace("</", "")
            ),
            T::Empty => String::new(),
            _ => format!("ComponentName::{:?}", self.component),
        }
    }

    /// Render the component as HTML. Since this is a profile layout, we require
    /// a reference to the [`Profile`] this layout is being rendered for.
    pub fn render_with_junk(
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
        if !self.json.is_empty() {
            return self.fill().render_with_junk(
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
            );
        }

        // regular
        match self.component {
            T::Flex => format!(
                "<div class=\"flex {} {} {} {} {}\" style=\"{}\" id=\"{}\">{}</div>",
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
                self.option("class", None),
                self.option("style", None),
                self.option("id", None),
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
            T::Divider => format!("<hr class=\"{}\" />", {
                let class = self.option("class", None);
                if !class.is_empty() {
                    class
                } else {
                    String::new()
                }
            }),
            T::Markdown => format!(
                "<div class=\"{}\">{}</div>",
                self.option("class", None),
                rainbeam_shared::ui::render_markdown(&self.option("text", None))
            ),
            T::Style => format!(
                "<style>{}</style>",
                self.option("data", None).replace("</", "")
            ),
            T::Empty => String::new(),
            _ => format!("ComponentName::{:?}", self.component),
        }
    }

    /// Render the component to block format. This format doesn't show the fully
    /// rendered form of the layout, but instead just blocks which represent the
    /// component.
    ///
    /// This rendering is used in the editor because it saves so many server resources.
    /// The normal rendering eats memory, as it recursively renders the same HTML template.
    ///
    /// The only component rendered halfway normally as a block is [`ComponentName::Flex`] components.
    pub fn render_block(&self) -> String {
        use ComponentName as T;

        match self.component {
            T::Flex => format!(
                "<div data-component-name=\"{:?}\" class=\"layout_editor_block flex {} {} {} {} {}\">{}</div>",
                self.component,
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
                {
                    let mobile = self.option("mobile", None);
                    if !mobile.is_empty() {
                        format!("sm:{mobile}")
                    } else {
                        String::new()
                    }
                },
                {
                    let mut children: String = String::new();

                    for child in &self.children {
                        children.push_str(&child.render_block());
                    }

                    children
                }
            ),
            _ => format!(
                "<div class=\"layout_editor_block {}\" data-component-name=\"{:?}\">{:?} ({}b)</div>",
                if self.component == T::Markdown {
                    "w-full"
                } else {
                    ""
                },
                self.component, self.component, {
                    let mut size: usize = 0;

                    for option in &self.options {
                        size += option.0.len() + option.1.len()
                    }

                    size
                }
            ),
        }
    }

    /// Render the component to tree format (using HTML `<details>`).
    pub fn render_tree(&self) -> String {
        // rustfmt has left as, as usual
        let tag = if self.children.len() == 0 {
            "div"
        } else {
            "details"
        };

        format!(
            "<{tag} class=\"layout_editor_tree_block flex flex-col gap-2 w-full\" data-component-name=\"{:?}\">
                <summary><b>{:?} ({}b)</b></summary>
                {}
            </{tag}>",
            self.component,
            self.component,
            {
                let mut size: usize = 0;

                for option in &self.options {
                    size += option.0.len() + option.1.len()
                }

                size
            },
            {
                let mut children: String = String::new();

                for child in &self.children {
                    children.push_str(&child.render_tree());
                }

                children
            }
        )
    }
}

impl AsRef<LayoutComponent> for LayoutComponent {
    fn as_ref(&self) -> &LayoutComponent {
        // the fact that this is a fix is crazy
        self
    }
}
