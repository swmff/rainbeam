# Langbeam

Versioned l10n (localization) files.

## `LangFile`

Text is stored in a "LangFile". In a LangFile, you must define a `name`, `version`, and the `data` stored with in.

The `name` of a LangFile follow this format:

```
{reverse-domain-name-notation id}:{ISO 639 language code}-{ISO 3166-1 alpha-2 country code}
```

The `version` of a LangFile should follow semantic versioning.

An example LangFile may look similar to this:

```json
{
    "name": "net.rainbeam.langs:en-US",
    "version": "1.0.0",
    "data": {
        "example_label": "Example Label"
    }
}
```

The key of entries in `data` should always stay the same. Only the value of entries should be translated between language files.

## `langs` directory

All language files should be pulled from `{cwd}/langs`. The file name of files does not matter, as the library only cares about the `name` field of each of the files. No files that aren't JSON files are allowed in this directory.
