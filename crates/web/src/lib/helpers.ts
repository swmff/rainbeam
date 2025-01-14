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

export function clean(s: Serialized | Array<any>) {
    if (!s) {
        return s;
    }

    const bad = [
        "ips",
        "ip",
        "tokens",
        "token_context",
        "password",
        "salt",
        "coins",
        "badges",
        "group",
        "tier",
        "email",
        "policy_consent"
    ];

    if (typeof s === "object") {
        let s_ = s as Serialized;
        for (const field of Object.entries(s)) {
            const field_type = typeof field;

            if (!bad.includes(field[0])) {
                if (field_type === "object") {
                    s_[field[0]] = clean(field[1]);
                }

                continue;
            }

            if (Array.isArray(field[1])) {
                s_[field[0]] = [];
            } else if (field_type === "string") {
                s_[field[0]] = "";
            } else if (field_type === "number") {
                s_[field[0]] = 0;
            } else if (field_type === "object") {
                s_[field[0]] = {};
            }
        }

        s = s_;
    } else if (Array.isArray(s)) {
        let s_ = s as Array<any>;

        for (const item of s_) {
            if (typeof item === "object") {
                s_.splice(s_.indexOf(item), 1, clean(item));
            }
        }

        s = s_;
    }

    return s;
}
