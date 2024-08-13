# 🎇 Sparkler

A simple [Retrospring](https://github.com/Retrospring/retrospring)-inspired service.

## Usage

To clone Sparkler, please clone **with submodules**:

```bash
git clone --recurse-submodules https://github.com/swmff/sparkler
```

Sparkler requires h-captcha to secure logins and registers. You can provide your h-captcha configuration in `~/.config/xsu-apps/sparkler/config.toml`:

```toml
# ...
[captcha]
site_key = "..."
secret = "..."
```

For Sparkler to properly serve static assets, you must link the `static` directory to `~/.config/xsu-apps/sparkler/static`:

```bash
ln -s /path/to/xsu/crates/sparkler/static ~/.config/xsu-apps/sparkler/static
```

You can provide a Markdown file for `/site/about` by creating `static/site/about.md`.

## Authentication

Sparkler requires a [`xsu-authman`](https://github.com/hkauso/xsu) connection to provide authentication.

Users with the group permission "Manager" will be able to manage responses and view reports.
