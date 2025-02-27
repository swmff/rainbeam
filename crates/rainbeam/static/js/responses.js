(() => {
    const self = reg_ns("responses", ["app"]);

    self.define(
        "create",
        async function (
            { $, app },
            question,
            content,
            tags = "",
            warning = "",
            reply = "",
            unlisted = false,
            circle = "",
        ) {
            await app.debounce("responses:create");
            if (!tags) {
                tags = "";
            }

            return new Promise((resolve, reject) => {
                fetch("/api/v1/responses", {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json",
                    },
                    body: JSON.stringify({
                        question,
                        content,
                        tags:
                            tags === ""
                                ? []
                                : tags.split(",").map((t) => t.trim()),
                        warning: warning || "",
                        reply: reply || "",
                        unlisted: unlisted || false,
                        circle: circle || "",
                    }),
                })
                    .then((res) => res.json())
                    .then((res) => {
                        const is_post = question === "0";

                        if (res.success === false) {
                            app.toast("error", res.message);
                            return reject(res);
                        }

                        if (!is_post) {
                            app.smooth_remove(
                                document.getElementById(`question:${question}`),
                                500,
                            );
                        }

                        return resolve(res);
                    });
            });
        },
    );

    self.define("edit", function ({ $, app }, id, content) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/responses/${id}`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    content,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Response edited!" : res.message,
                    );

                    if (res.success === true) {
                        return resolve(res);
                    }

                    return reject(res);
                });
        });
    });

    self.define("edit_tags", function ({ $, app }, id, tags) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/responses/${id}/tags`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    tags,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Response edited!" : res.message,
                    );

                    if (res.success === true) {
                        return resolve(res);
                    }

                    return reject(res);
                });
        });
    });

    self.define("edit_context", function ({ $, app }, id, context) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/responses/${id}/context`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    context,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Response edited!" : res.message,
                    );

                    if (res.success === true) {
                        return resolve(res);
                    }

                    return reject(res);
                });
        });
    });

    self.define("edit_context_warning", function ({ $, app }, id, warning) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/responses/${id}/context/warning`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    warning,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Response edited!" : res.message,
                    );

                    if (res.success === true) {
                        return resolve(res);
                    }

                    return reject(res);
                });
        });
    });

    self.define("delete", async function ({ $, app }, id) {
        if (
            !(await trigger("app::confirm", [
                "Are you sure you want to do this? This will delete the response and its question.",
            ]))
        ) {
            return;
        }

        fetch(`/api/v1/responses/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Response deleted!" : res.message,
                );

                app.smooth_remove(
                    document.getElementById(`response:${id}`),
                    500,
                );
            });
    });

    self.define("unsend", async function ({ $, app }, id) {
        if (
            !(await trigger("app::confirm", [
                "Are you sure you want to do this? This will delete the response and allow you to answer the question again.",
            ]))
        ) {
            return;
        }

        fetch(`/api/v1/responses/${id}/unsend`, {
            method: "POST",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Question returned to inbox!" : res.message,
                );

                app.smooth_remove(
                    document.getElementById(`response:${id}`),
                    500,
                );
            });
    });

    self.define(
        "gen_share",
        function (_, target, id, target_length, include_link) {
            const share_hashtag =
                globalThis.__user.metadata.kv["rainbeam:share_hashtag"] || "";

            // resolve target
            while (!target.classList.contains("response")) {
                target = target.parentElement;
            }

            const part_1 = (
                target.querySelector(".question_content p:nth-child(2)") || {
                    innerText: "",
                }
            ).innerText;

            const part_2 = target.querySelector(
                ".response_content:not(include-partial *) p",
            ).innerText;

            // ...
            const link =
                include_link !== false
                    ? `${window.location.origin}/+r/${id}`
                    : "";

            const link_size = link.length;
            target_length -= link_size;

            let out = "";
            const separator = " — ";

            const part_2_size = target_length / 2 - 1;
            const sep_size = separator.length;
            const part_1_size = target_length / 2 - sep_size;

            if (share_hashtag) {
                out += `#${share_hashtag}`;
            }

            if (part_1 !== "") {
                out +=
                    part_1_size > part_1.length
                        ? part_1
                        : part_1.substring(0, part_1_size);

                out += separator;
            }

            if (part_2 !== "") {
                out +=
                    part_2_size > part_2.length
                        ? part_2
                        : part_2.substring(0, part_2_size);
            }

            out += ` ${link}`;
            return out;
        },
    );

    self.define("click", function (_, id, do_render_nested) {
        // close dropdowns
        trigger("app::hooks::dropdown.close");

        // check for warning
        const warning = document.querySelector(
            `#response\\:${id} .response_warning`,
        );

        if (warning) {
            const content = document.querySelector(
                `#response\\:${id} .response_content`,
            );

            const question = document.querySelector(
                `#response\\:${id} .question`,
            );

            if (question) {
                question.classList.remove("hidden");
            }

            content.classList.remove("hidden");
            warning.classList.add("hidden");
        }

        // ...
        if (!do_render_nested) {
            window.location.href = `/response/${id}`;
            return;
        }
    });
})();
