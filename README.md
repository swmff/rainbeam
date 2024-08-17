<h1 align="center">🎇 Sparkler</h1>

Sparkler is a simple Q&A social network designed for community! On sparkler, users can create and customize their profile to look how they want, and then other users can ask them questions using their account or an anonymous username. Users receive questions in their inbox and can then answer them or manage them. Users can also ask “global questions” which everybody who is following them can see in a specialized timeline. Global questions can be answered by any logged-in user. Users can also respond to existing responses with comments to further interact with their friends' responses!

## Usage

To start, clone the Sparkler repository and build it:

```bash
git clone --recurse-submodules https://github.com/swmff/sparkler
cd sparkler
just build sqlite
```

Instead of “sqlite”, you can also use “mysql" or "postgres” for MySQL/MariaDB and PostgreSQL respectively!

### hCaptcha

Sparkler requires hCaptcha to secure logins and registers. You can provide your h-captcha configuration in `~/.config/xsu-apps/sparkler/config.toml`:

```toml
# ...
[captcha]
site_key = "..."
secret = "..."
```

You can sign up for an hCaptcha account at <https://www.hcaptcha.com/>!

### Static assets

For Sparkler to properly serve static assets, you must link the `static` directory to `~/.config/xsu-apps/sparkler/static`:

```bash
ln -s /path/to/xsu/crates/sparkler/static ~/.config/xsu-apps/sparkler/static
```

You can provide a Markdown file for `/site/about` by creating `static/site/about.md`. This file can be used to provide information about your specific instance!

### Moderation

After you have created your first account, you'll need to manually create a permissions group in the database so that you can mark the account as a `Manager`. The manager permission allows you to delete accounts, responses, comments, and questions. You can also view profile warnings and reports with this permission. You can view an example SQL query to do this [here](https://github.com/swmff/sparkler/blob/master/sql/manager.sql)!

## Contributing

You can view information about contributing to Sparkler [here](https://github.com/swmff/sparkler/blob/master/CONTRIBUTING.md), as well as the contributor code of conduct [here](https://github.com/swmff/sparkler/blob/master/CODE_OF_CONDUCT.md)!
