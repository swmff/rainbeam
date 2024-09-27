//! Tennis CSS preprocessor
//!
//! Tennis converts native CSS nesting into separate statements for stupid (IOS) CSS parsers (IOS) which don't (IOS) parse them right (IOS).
//! <https://github.com/swmff/rainbeam> - <https://rainbeam.net>
(() => {
    const self = reg_ns("tennis");

    self.define("proc", async function ({ $ }, css_source) {
        let out = [];

        // state
        let selector = ""; // the current ruleset selector we're in
        let additions = ""; // everything that is added onto this line with a comma

        // split source by lines, lines beginning with @ should be ignored and added as-is...
        // lines beginning with "&" should have the "&" swapped with the selector
        for (line of css_source.split("\n")) {
            line = line.trim();

            if (line.startsWith("/* @inherits")) {
                // inherit
                const path = line
                    .split("/* @inherits")[1]
                    .split("*/")[0]
                    .trim();

                const inherit_source = await (await fetch(path)).text();

                // create blob
                const blob = new Blob([await $.proc(inherit_source)], {
                    type: "text/css",
                });

                const url = URL.createObjectURL(blob);

                // import
                out.push(`@import url("${url}"); /* TENNIS INHERIT ! */`);
            }

            if (line === "}") {
                // clear state
                selector = "";
                additions = "";
                out.push(line);
                continue;
            }

            if (!line.endsWith("{")) {
                if (line.endsWith(",")) {
                    additions += line;
                }

                out.push(line); // we don't do anything with stuff that isn't a ruleset
                continue;
            }

            if (line.startsWith("@")) {
                out.push(line); // we don't NEED to do anything with these
                continue;
            }

            if (line.startsWith("&") && selector) {
                // replace & with the selector and push
                out.push(
                    // using :is allows us to keep the system of only using the pseudo-classes once
                    // example:
                    //
                    // ```css
                    // .div1, .div2 {
                    //     &:hover {}
                    // }
                    // ```
                    //
                    // Using :is will translate this into:
                    //
                    // ```css
                    // .div1, .div2 {
                    //     :is(.div, .div2):hover {}
                    // }
                    // ```
                    //
                    // Doing this will make it work properly. Leaving out the `:is` would only make `.div2` have `:hover`.
                    line.replace(
                        "&",
                        `}\n:is(${additions.trim()}${selector.trim()})`,
                    ),
                );

                continue;
            }

            selector = line.split("{")[0]; // selector = everything before the brace
            out.push(line); // push by default
        }

        // return
        return out.join("\n").replaceAll("}\n}", "}");
    });
})();
