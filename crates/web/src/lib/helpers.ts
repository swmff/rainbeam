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
