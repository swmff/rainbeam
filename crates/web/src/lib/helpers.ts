import { None, type Option } from "./classes/Option";
import type { Serialized } from "./proc/tserde";

/// https://swmff.github.io/rainbeam/authbeam/model/struct.Profile.html#method.anonymous_tag
export function anonymous_tag(
    input: string
): [boolean, string, string, string] {
    if (input !== "anonymous" && !input.startsWith("anonymous#")) {
        // not anonymous
        return [false, "", "", input];
    }

    // anonymous questions from BEFORE the anonymous tag update will just have the "anonymous" tag
    const split = input.split("#");
    return [true, split[1] || "unknown", split[0], input];
}

import { marked } from "marked";
export function render_markdown(input: string): string {
    return marked.parse(input) as string;
}

export function closure<T>(run: () => T): T {
    return run();
}

export async function aclosure<T>(run: () => T): Promise<T> {
    return await run();
}

export const BAD_ITEMS = [
    "ips",
    "ip",
    "tokens",
    "token_context",
    "password",
    "salt",
    "coins",
    "group",
    "email",
    "policy_consent"
];

export const BAD_ITEMS_POWERFUL = [];

export function clean(s: Serialized, bad: Array<string> = BAD_ITEMS) {
    if (!s) {
        return s;
    }

    const metadata_allowed = ["sparkler:display_name"];

    for (const field of Object.entries(s)) {
        const field_type = typeof field[1];

        if (!bad.includes(field[0])) {
            if (field[0] === "kv") {
                const f: Serialized = {};
                for (const f_ of Object.entries(field[1])) {
                    if (!metadata_allowed.includes(field[0])) {
                        continue;
                    }

                    f[f_[0]] = f_[1];
                }

                s[field[0]] = f;
            } else if (field_type === "object") {
                s[field[0]] = clean(field[1], bad);
            } else if (field[0] === "id") {
                // remove anonymous tags
                s[field[0]] = field[1].split("#")[0];
            }

            continue;
        }

        if (Array.isArray(field[1])) {
            s[field[0]] = [];
        } else if (field_type === "string") {
            s[field[0]] = "";
        } else if (field_type === "number") {
            s[field[0]] = 0;
        } else if (field_type === "object") {
            s[field[0]] = {};
        }
    }

    return s;
}
