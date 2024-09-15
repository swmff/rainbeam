# Neospring Syndication

NSS (Neospring Syndication) is a system for sharing content between Neospring instances using simple XML files. NSS is based on the [Atom Syndication Format](https://datatracker.ietf.org/doc/html/rfc4287) with some minor modifications.

## Feeds

Each feed element contains some basic information to describe it.

Feeds can exist either in the `<feed>` root element, or the `<subfeed>` element.

### Elements

#### `<subfeed>`

A subfeed is a separate feed that is part of the main feed. A `<feed>` represents the root feed object, but a `<subfeed>` represents a smaller collection of `<entry>` elements.

```xml
<subfeed name="subfeed_name">...</subfeed>
```

A subfeed is required to have a `name` attribute to describe what the feed is for.

#### `<title>`

The `title` is used to set the title of the feed.

```xml
<title>feed title</title>
```

#### `<subtitle>` (optional)

A secondary title element rendered under the title.

```xml
<subtitle>extra titles for the feed</subtitle>
```

#### `<icon>` (optional)

A link to the avatar icon shown for the feed. This is best used for rendering profiles with a separate profile picture and banner.

```xml
<icon>https://example.com/@example/avatar</icon>
```

#### `<banner>` (optional)

A link to the banner icon shown for the feed. This is shown alongside the icon.

```xml
<banner>https://example.com/@example/banner</banner>
```

#### `<link>` (optional)

A `link` can be used to render links alongside the feed. Their `rel` attribute determines the text of the link.

`rel="alternate"` should be used to include a link to an alternate version of the same feed.

```xml
<link rel="alternate" type="text/html" href="https://example.com/@example" />
```

#### `<description>`

A `description` is used to include extra information about the feed and is generally shown under the title/subtitle.

```xml
<description>extra information blah blah blah</description>
```

## Entries

Individual entries are denoted by an `<entry>` element in the feed/subfeed.

### Elements

Each element here is also allowed to be included in the root `<feed>` (NOT subfeeds). There should only be allowed to be one of each element included in the root `<feed>`, but any amount is allowed inside an `<entry>`.

#### `<entry type="question">`

The question being responded to in this entry.

A question needs the following elements:

* `id` - the unique ID of the question
* `author` - the username of the question author
* `content` - the content of the question
* `timestamp` - a UNIX epoch timestamp number


#### `<entry type="response">`

The responses responding to the question.

A response needs the following elements:

* `id` - the unique ID of the response
* `author` - the username of the response author
* `content` - the content of the response
* `timestamp` - a UNIX epoch timestamp number
* `comments` - the number of comments the response has
* `reactions` - the number of reactions the response has

The response should also have a `<link rel="comments" ... />` element to link to its comments.

#### `<entry type="comment">`

The comments commenting on the response element.

A comment needs the following elements:

* `id` - the unqiue ID of the comment
* `author` - the username of the comment author
* `content` - the content of the comment
* `timestamp` - a UNIX epoch timestamp number
* `replies` - the number of replies the comment has
* `reactions` - the number of reactions the comment has

The comment should also have a `<link rel="replies ... />` element to link to its replies.

#### `<entry type="user">`

* `id` - the unique ID of the user
* `name` - the name of the user
* `icon` - a link to the user's avatar
