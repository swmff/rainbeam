import { transform } from "lightningcss";
import swc from "@swc/core";

import process from "node:process";
import fs from "node:fs/promises";
import crypto from "node:crypto";
import child_process from "node:child_process";

function hash(input) {
    return crypto
        .createHash("sha256")
        .update(input)
        .digest("hex")
        .substring(0, 10);
}

function replace_vars(content, vars) {
    const regex = new RegExp(/(\{\{)\s*(var)\s*\"(.*?)\"\s*(\}\})/g);
    let groups = regex.exec(content);

    while (null !== groups) {
        if (vars[groups[3]]) {
            content = content.replace(groups[0], vars[groups[3]]);
        }

        groups = regex.exec(content);
    }

    return content;
}

export default async function build(options) {
    const __cwd = process.cwd();

    const commit = child_process
        .execSync("git rev-parse HEAD")
        .toString()
        .trim();

    const build_vars = {
        time: new Date().toISOString(),
        time_unix: new Date().getTime(),
        commit,
        commit_short: commit.substring(0, 10),
    };

    // create build directory if it doesn't already exist
    try {
        await fs.stat(`${__cwd}/${options.build_dir}`);
    } catch {
        await fs.mkdir(`${__cwd}/${options.build_dir}`);
    }

    // clear directories we're told to clear before build
    for (const dir of options.clear_build_dirs) {
        try {
            await fs.stat(`${__cwd}/${options.build_dir}/${dir}`);
            await fs.rm(`${__cwd}/${options.build_dir}/${dir}`, {
                recursive: true,
            });
        } catch {}
    }

    // create templates build directory if it doesn't already exist
    try {
        await fs.stat(`${__cwd}/${options.templates_build_dir}`);
    } catch {
        await fs.cp(
            `${__cwd}/${options.templates_dir}`,
            `${__cwd}/${options.templates_build_dir}`,
            { recursive: true },
        );
    }

    const hashed_files = {};

    // walk css_dir
    async function walk_dir(
        transform_callback,
        dir_root = __cwd,
        build_sub_dir = "",
        sub_dir = "",
        do_sub = true,
        do_hash = true,
        do_clear_dir = true,
    ) {
        try {
            await fs.stat(`${__cwd}/${options.build_dir}/${build_sub_dir}`);
        } catch {
            await fs.mkdir(`${__cwd}/${options.build_dir}/${build_sub_dir}`);
        }

        const files = await fs.readdir(`${dir_root}/${sub_dir}`);

        for (let file of files) {
            const full_path = `${dir_root}/${sub_dir}${file}`;
            const stat = await fs.stat(full_path);

            if (stat.isDirectory()) {
                console.log(`sub ${sub_dir}${file}`);

                if (do_sub) {
                    try {
                        await fs.stat(
                            `${__cwd}/${options.build_dir}/${build_sub_dir}${sub_dir}${file}`,
                        );
                    } catch {
                        await fs.mkdir(
                            `${__cwd}/${options.build_dir}/${build_sub_dir}${sub_dir}${file}`,
                        );
                    }
                }

                await walk_dir(
                    transform_callback,
                    dir_root,
                    build_sub_dir,
                    `${sub_dir}${file}/`,
                    do_sub,
                );
                continue;
            }

            if (do_hash) {
                const file_content = await fs.readFile(full_path, {
                    encoding: "utf8",
                });

                const hashed_file = `${hash(file_content)}.h.${file}`;
                hashed_files[file] = hashed_file;
                file = hashed_file;
            }

            const build_path = do_sub
                ? `${__cwd}/${options.build_dir}/${build_sub_dir}${sub_dir}${file}`
                : `${__cwd}/${options.build_dir}/${build_sub_dir}${file}`;

            await transform_callback(file, full_path, build_path);
        }
    }

    // walk css dir
    await walk_dir(
        async (file_name, full_path, build_path) => {
            // minify
            console.log(`min ${file_name}`);
            const { code } = transform({
                filename: file_name,
                code: Buffer.from(
                    await fs.readFile(full_path, { encoding: "utf8" }),
                ),
                minify: false,
                sourceMap: true,
            });

            let content = new TextDecoder().decode(code);

            // hashed files
            for (const hashed_file of Object.entries(hashed_files)) {
                content = content.replaceAll(
                    `@import "${hashed_file[0]}"`,
                    `@import "${hashed_file[1]}"`,
                );
            }

            // write
            await fs.writeFile(build_path, content);
        },
        `${__cwd}/${options.css_dir}`,
        "css/",
    );

    // walk js dir
    await walk_dir(
        async (file_name, full_path, build_path) => {
            // minify
            console.log(`min ${file_name}`);

            const compiled = await swc.transform(
                await fs.readFile(full_path, { encoding: "utf8" }),
                {
                    filename: file_name,
                    sourceMaps: true,
                    isModule: true,
                    jsc: {
                        parser: {
                            syntax: "ecmascript",
                            jsx: true,
                            autoAccessors: true,
                        },
                        transform: {},
                    },
                },
            );

            let { code } = await swc.minify(compiled.code, {
                compress: true,
                mangle: true,
                format: options.js_format_options || {},
            });

            // hashed files
            for (const hashed_file of Object.entries(hashed_files)) {
                code = code
                    .replaceAll(
                        `use("${hashed_file[0].replace(".js", "")}`,
                        `use("${hashed_file[1].replace(".js", "")}`,
                    )
                    .replaceAll(
                        `require("${hashed_file[0].replace(".js", "")}`,
                        `require("${hashed_file[1].replace(".js", "")}`,
                    );
            }

            // write
            await fs.writeFile(build_path, code);
        },
        `${__cwd}/${options.js_dir}`,
        "js/",
    );

    // walk templates dir to download icons
    const icons = options.extra_icon_imports || {};

    await walk_dir(
        async (file_name, full_path, _) => {
            // minify
            console.log(`template ${file_name}`);

            const content = await fs.readFile(full_path, { encoding: "utf8" });

            // with class
            const class_regex = new RegExp(
                /(\{\{)\s*(icon)\s*\"(.*?)\"\s*c\((.*?)\)\s*(\}\})/g,
            );

            let groups = class_regex.exec(content);
            while (null !== groups) {
                if (!icons.includes(groups[3])) {
                    icons.push(groups[3]);
                }

                groups = class_regex.exec(content);
            }

            // regular
            const regex = new RegExp(/(\{\{)\s*(icon)\s*\"(.*?)\"\s*(\}\})/g);
            let groups_ = regex.exec(content);

            while (null !== groups_) {
                if (!icons.includes(groups_[3])) {
                    icons.push(groups_[3]);
                }

                groups_ = regex.exec(content);
            }
        },
        `${__cwd}/${options.templates_dir}`,
        "icons/",
        "",
        false,
        false,
        false,
    );

    // download icons
    const icons_mem = {};
    const icons_endpoint =
        "https://raw.githubusercontent.com/lucide-icons/lucide/refs/heads/main/icons/";

    for (const icon of icons) {
        const file_path = `${__cwd}/${options.build_dir}/icons/${icon}.svg`;

        try {
            // if the file exists, don't fetch it
            console.log(`icon/check ${icon}`);

            await fs.stat(file_path);
            icons_mem[icon] = await fs.readFile(file_path, {
                encoding: "utf8",
            });
        } catch {
            console.log(`icon/save ${icon}`);

            const text = await (
                await fetch(`${icons_endpoint}${icon}.svg`)
            ).text();

            await fs.writeFile(file_path, text);
            icons_mem[icon] = text;

            console.log(`icon/finish ${icon}`);
        }
    }

    // walk templates dir to replace icons
    await walk_dir(
        async (file_name, full_path, _) => {
            // minify
            console.log(`template(2) ${file_name}`);
            let content = await fs.readFile(full_path, { encoding: "utf8" });

            content = replace_vars(content, build_vars);

            // text
            const text_regex = new RegExp(
                /(\{\{)\s*(text)\s*\"(.*?)\"\s*(\}\})/g,
            );

            let groups_t = text_regex.exec(content);
            while (null !== groups_t) {
                const text = `\n= lang.get("${groups_t[3]}")`;
                content = content.replace(groups_t[0], text);
                groups_t = text_regex.exec(content);
            }

            // selector with class
            const class_regex = new RegExp(
                /(\{\{)\s*(icon)\s*\"(.*?)\"\s*c\((.*?)\)\s*(\}\})/g,
            );

            let groups = class_regex.exec(content);
            while (null !== groups) {
                const icon_text = icons_mem[groups[3]].replace(
                    "<svg",
                    `<svg class="icon ${groups[4]}"`,
                );

                content = content.replace(
                    groups[0],
                    `%r:\n${icon_text.replaceAll("\n", " ")}\n%-r:`,
                );
                groups = class_regex.exec(content);
            }

            // regular selector
            const regular_regex = new RegExp(
                /(\{\{)\s*(icon)\s*\"(.*?)\"\s*(\}\})/g,
            );

            let groups_ = regular_regex.exec(content);
            while (null !== groups_) {
                const icon_text = icons_mem[groups_[3]].replace(
                    "<svg",
                    '<svg class="icon"',
                );

                content = content.replace(
                    groups_[0],
                    `%r:\n${icon_text.replaceAll("\n", " ")}\n%-r:`,
                );
                groups_ = regular_regex.exec(content);
            }

            // hashed files
            for (const hashed_file of Object.entries(hashed_files)) {
                content = content
                    .replaceAll(
                        `src="/static/build/js/${hashed_file[0]}"`,
                        `src="/static/build/js/${hashed_file[1]}"`,
                    )
                    .replaceAll(
                        `href="/static/build/css/${hashed_file[0]}"`,
                        `href="/static/build/css/${hashed_file[1]}"`,
                    )
                    .replaceAll(
                        `use("${hashed_file[0].replace(".js", "")}`,
                        `use("${hashed_file[1].replace(".js", "")}`,
                    )
                    .replaceAll(
                        `require("${hashed_file[0].replace(".js", "")}`,
                        `require("${hashed_file[1].replace(".js", "")}`,
                    );
            }

            // save file
            await fs.writeFile(
                full_path.replace(
                    `${__cwd}/${options.templates_dir}`,
                    `${__cwd}/${options.templates_build_dir}`,
                ),
                content,
            );
        },
        `${__cwd}/${options.templates_dir}`,
        "",
        "",
        false,
        false,
        false,
    );
}
