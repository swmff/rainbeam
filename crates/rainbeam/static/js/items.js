(() => {
    const self = reg_ns("items", ["app"]);

    self.define(
        "create",
        function ({ $, app }, name, description, content, cost, type) {
            return new Promise((resolve, reject) => {
                fetch("/api/v0/auth/items", {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json",
                    },
                    body: JSON.stringify({
                        name,
                        description,
                        content,
                        cost,
                        type,
                    }),
                })
                    .then((res) => res.json())
                    .then((res) => {
                        app.toast(
                            res.success ? "success" : "error",
                            res.success ? "Item created!" : res.message,
                        );

                        if (res.success === true) {
                            resolve(res.payload);
                        } else {
                            reject();
                        }
                    });
            });
        },
    );

    self.define("content_input", async function ({ _, app }, type) {
        let content = "";

        if (type === "UserTheme") {
            if (
                !(await app.confirm(
                    "Are you sure you would like to create an item using your current profile theme?\n\nYou can press no to input CSS directly instead.",
                ))
            ) {
                const css = await app.prompt_long("Enter CSS manually:");

                if (!css) {
                    return;
                }

                return css;
            }

            // fetch current user profile
            const me = await (await fetch("/api/v0/auth/me")).json();

            if (!me.success) {
                return alert(me.message || "Failed to fetch self.");
            }

            // compile the fields we want
            let content_root = ":root,*{"; // sparkler:color_*
            let content_custom = ""; // sparkler:custom_css

            for (const field of Object.entries(me.payload.metadata.kv)) {
                if (!field[1]) {
                    continue;
                }

                if (field[0].startsWith("sparkler:color_")) {
                    content_root += `/* ${field[0]} */\n--${field[0].replace("sparkler:", "").replaceAll("_", "-")}:${field[1]} !important;\n`;
                } else if (field[0] === "sparkler:custom_css") {
                    content_custom = field[1];
                } else {
                    console.log(`skip ${field[0]}`);
                }
            }

            content = `${content_root}}\n/* sparkler:custom_css */\n${content_custom}`;
            console.log("content compiled");
        } else if (type === "Text") {
            content = await app.prompt_long("Item text:");

            if (!content) {
                return;
            }
        } else if (type === "Module") {
            content = await app.prompt(
                "WASM module checksum (this cannot be changed):",
            );

            if (!content) {
                return;
            }
        } else {
            return;
        }

        return content;
    });

    self.define("delete", async function ({ $, app }, id) {
        if (
            !(await trigger("app:confirm", [
                "Are you sure you want to do this?",
            ]))
        ) {
            return;
        }

        fetch(`/api/v0/auth/item/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Item deleted!" : res.message,
                );
            });
    });

    self.define("purchase", async function ({ $, app }, id, price) {
        if (
            !(await trigger("app:confirm", [
                `Are you sure you want to purchase this item for ${price} coins?`,
            ]))
        ) {
            return;
        }

        fetch(`/api/v0/auth/item/${id}/buy`, {
            method: "POST",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Item purchased!" : res.message,
                );
            });
    });

    self.define(
        "edit",
        async function ({ $, app }, id, name, description, cost) {
            fetch(`/api/v0/auth/item/${id}`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    name,
                    description,
                    cost,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Item updated!" : res.message,
                    );
                });
        },
    );

    self.define("edit_content", async function ({ $, app }, id, type) {
        const content = await $.content_input(type);

        fetch(`/api/v0/auth/item/${id}/content`, {
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
                    res.success ? "Item updated!" : res.message,
                );
            });
    });

    self.define("status", async function ({ $, app }, id, status) {
        fetch(`/api/v0/auth/item/${id}/status`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                status,
            }),
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Item updated!" : res.message,
                );

                if (res.success) {
                    for (const element of Array.from(
                        document.querySelectorAll("[data-item-status]"),
                    )) {
                        element.classList.remove("active");
                    }

                    document
                        .querySelector(`[data-item-status="${status}"]`)
                        .classList.add("active");
                }
            });
    });
})();
