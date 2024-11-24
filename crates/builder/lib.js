import { transform } from "lightningcss";
import swc from "@swc/core";

import process from "node:process";
import fs from "node:fs/promises";

export default async function build(options) {
    const __cwd = process.cwd();

    // create build directory if it doesn't already exist
    try {
        await fs.stat(`${__cwd}/${options.build_dir}`);
    } catch {
        await fs.mkdir(`${__cwd}/${options.build_dir}`);
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

    // walk css_dir
    async function walk_dir(
        transform_callback,
        dir_root = __cwd,
        build_sub_dir = "",
        sub_dir = "",
        do_sub = true,
    ) {
        try {
            await fs.stat(`${__cwd}/${options.build_dir}/${build_sub_dir}`);
        } catch {
            await fs.mkdir(`${__cwd}/${options.build_dir}/${build_sub_dir}`);
        }

        const files = await fs.readdir(`${dir_root}/${sub_dir}`);

        for (const file of files) {
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

            await transform_callback(
                file,
                full_path,
                do_sub
                    ? `${__cwd}/${options.build_dir}/${build_sub_dir}${sub_dir}${file}`
                    : `${__cwd}/${options.build_dir}/${build_sub_dir}${file}`,
            );
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

            await fs.writeFile(build_path, code);
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

            const { code } = await swc.minify(compiled.code, {
                compress: true,
                mangle: true,
                format: options.js_format_options || {},
            });

            await fs.writeFile(build_path, code);
        },
        `${__cwd}/${options.js_dir}`,
        "js/",
    );

    // walk templates dir to download icons
    const icons = [];

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

                content = content.replace(groups[0], icon_text); // replace icon element with svg
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

                content = content.replace(groups_[0], icon_text); // replace icon element with svg
                groups_ = regular_regex.exec(content);
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
    );
}
