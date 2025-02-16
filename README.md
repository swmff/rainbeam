<h1 align="center">🐈‍⬛ Rainbeam</h1>

Rainbeam is a simple Q&A social network designed for community! On Rainbeam, users can create and customize their profile to look how they want, and then other users can ask them questions using their account or an anonymous username. Users receive questions in their inbox and can then answer them or manage them. Users can also ask “global questions” which everybody who is following them can see in a specialized timeline. Global questions can be answered by any logged-in user. Users can also respond to existing responses with comments to further interact with their friends' responses!

## Repository structure

All core packages are contained in the `crates/` directory:

- `crates/builder/` - Client asset builder/bundler
- `crates/shared/` - Core shared utilities
- `crates/langbeam/` - l10n service
- `crates/databeam/` - Database connection manager
- `crates/authbeam/` - Authentication backend and API routes
- **(v)** `crates/rainbeam-core/` - Rainbeam database connection and types
    - `crates/rb/` - Rainbeam Axum routes (API and all pages)
    - `crates/rainbeam/` - Rainbeam server binary

Packages labeled with `(v)` are _version-tied_. This means that it and all the packages nested under it in the list share the same crate version.

## Usage

To start, clone the Rainbeam repository and build it:

```bash
git clone https://github.com/swmff/rainbeam
cd rainbeam
```

The Rainbeam server is built for Linux systems. Development is possible on Windows, but it is recommended that production servers be run on Linux amd64/aarch64 systems.

You'll need [just](https://just.systems/man/en/introduction.html) for many steps of our build process. You'll also need cargo with rustc version 1.83 minimum. Any recent Node.js version will also be needed.

To initialize the build system, you'll need to run `just init-builder`. This will install all JavaScript dependencies needed to build Rainbeam.

After you've initialized the builder, you can run `just build sqlite(/mysql/postgres)` to build the main server binary:

```bash
just build sqlite
```

Instead of “sqlite”, you can also use “mysql" or "postgres” for MySQL/MariaDB and PostgreSQL respectively!

To configure Rainbeam, create a `config.toml` file in `./.config` and `./.config/databeam`. You can copy the example files and edit them to your needs.

```bash
cp ./.config/config.example.toml ./.config/config.toml
cp ./.config/databeam/config.example.toml ./.config/databeam/config.toml
```

All configuration files for `./.config/config.toml` (the main configuration file) are available [here](https://swmff.github.io/rainbeam/rainbeam/config/struct.Config.html). You can find the databeam (database connection) configuration file options [here](https://swmff.github.io/rainbeam/databeam/sql/struct.DatabaseOpts.html).

### Updating

Most updates are as easy as just running `git pull`, however some require some manual work. Any update that changes the database stucture will require you to manually replicate these changes in your database.

All commits which do this will have an associated `.sql` file in `sql/` which will replicate this change. Some of these files may specify different SQL for PostgreSQL/MySQL/SQLite, however most won't. The name of each file specifies what it does, so use that to figure out which one to use after pulling.

### Configuration

You can configure Rainbeam in the configuration file located at `./.config/config.toml`. This file will be created for you when the server is first run.

#### Tier features

You can lock a given set of features behind the `tier` column of profiles using the `tiers` configuration key.

```toml
[tiers]
double_limits = 2
avatar_crown = 1
profile_badge = 1
# ...
```

The `tiers` configuration key contains a map of features where the key is the feature name, and the value is the minimum required tier.

By default, every benefit is at tier `1`.

### hCaptcha

Rainbeam requires hCaptcha to secure logins and registers. You can provide your h-captcha configuration in `./.config/config.toml`:

```toml
# ...
[captcha]
site_key = "..."
secret = "..."
```

You can sign up for an hCaptcha account at <https://www.hcaptcha.com/>!

### Moderation

Permissions are expressed using groups and flags. You can read more about user permissions (and groups) in the [book](https://swmff.github.io/rainbeam_staff_guide/technical/permissions.html)!

### Account registration

To open your instance up for free account registration, you'll need to set `registration_enabled` to `true` in `./.config/config.toml`.

### PWA

You are responsible for adding your own `manifest.json` file in the `static/` directory.

You can place your PWA logo images in `static/images/logo/`. This directory is ignored by git.

## Contributing

You can view information about contributing to Rainbeam [here](https://github.com/swmff/rainbeam/blob/master/.github/CONTRIBUTING.md), as well as the contributor code of conduct [here](https://github.com/swmff/rainbeam/blob/master/.github/CODE_OF_CONDUCT.md)!

You can view security information [here](https://github.com/swmff/rainbeam/blob/master/SECURITY.md).

## Attribution

Rainbeam is a faithful recreation of the amazing Q&A site [Retrospring](https://github.com/Retrospring/retrospring). Rainbeam uses many Retrospring's core concepts, ideas, and designs, however it is built from the ground up and uses none of Retrospring's original code or assets.
