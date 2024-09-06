<h1 align="center">🌼 Neospring</h1>

Neospring is a simple Q&A social network designed for community! On Neospring, users can create and customize their profile to look how they want, and then other users can ask them questions using their account or an anonymous username. Users receive questions in their inbox and can then answer them or manage them. Users can also ask “global questions” which everybody who is following them can see in a specialized timeline. Global questions can be answered by any logged-in user. Users can also respond to existing responses with comments to further interact with their friends' responses!

## Usage

To start, clone the Neospring repository and build it:

```bash
git clone --recurse-submodules https://github.com/swmff/neospring
cd neospring
just build sqlite
```

Instead of “sqlite”, you can also use “mysql" or "postgres” for MySQL/MariaDB and PostgreSQL respectively!

### hCaptcha

Neospring requires hCaptcha to secure logins and registers. You can provide your h-captcha configuration in `~/.config/xsu-apps/neospring/config.toml`:

```toml
# ...
[captcha]
site_key = "..."
secret = "..."
```

You can sign up for an hCaptcha account at <https://www.hcaptcha.com/>!

### Static assets

For Neospring to properly serve static assets, you must link the `static` directory to `~/.config/xsu-apps/neospring/static`:

```bash
ln -s /path/to/neospring/static ~/.config/xsu-apps/neospring/static
```

You can provide a Markdown file for `/site/about` by creating `static/site/about.md`. This file can be used to provide information about your specific instance!

### Moderation

After you have created your first account, you'll need to manually create a permissions group in the database so that you can mark the account as a `Manager`. The manager permission allows you to delete accounts, responses, comments, and questions. You can also view profile warnings and reports with this permission. You can view an example SQL query to do this [here](https://github.com/swmff/neospring/blob/master/sql/moderation.sql)!

You can add additional moderators with the `Helper` role. They can also be given `Manager`, however `Helper` is much better if you want them to have limited moderation abilities.

### Account registration

To open your instance up for free account registration, you'll need to set `registration_enabled` to `true` in `~/.config/xsu-apps/neospring/config.toml`.

## Contributing

You can view information about contributing to Neospring [here](https://github.com/swmff/neospring/blob/master/.github/CONTRIBUTING.md), as well as the contributor code of conduct [here](https://github.com/swmff/neospring/blob/master/.github/CODE_OF_CONDUCT.md)!

You can view security information [here](https://github.com/swmff/neospring/blob/master/SECURITY.md).

## Attribution

Neospring is a faithful recreation of the amazing Q&A site [Retrospring](https://github.com/Retrospring/retrospring). Neospring uses many Retrospring's core concepts, ideas, and designs, however it is built from the ground up and uses none of Retrospring's original code or assets.

Neospring uses [Twemoji](https://github.com/jdecked/twemoji) on its HTML pages for uniform emojis.
