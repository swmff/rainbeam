# 🎇 Sparkler

A simple [Retrospring](https://github.com/Retrospring/retrospring)-inspired service.

## Usage

For Sparkler to properly serve static assets, you must link the `static` directory to `~/.config/xsu-apps/sparkler/static`:

```bash
ln -s /path/to/xsu/crates/sparkler/static ~/.config/xsu-apps/sparkler/static
```

You can provide a Markdown file for `/site/about` by creating `static/site/about.md`.

## Authentication

Sparkler requires a [`xsu-authman`](https://github.com/hkauso/xsu) connection to provide authentication.

Users with the group permission "Manager" will be able to manage responses and view reports.
