(() => {
    const self = reg_ns("responses", ["app"]);

    self.define("create", function ({ $, app }, question, content, tags) {
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
                        tags === "" ? [] : tags.split(",").map((t) => t.trim()),
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    const is_post = question === "0";

                    app.toast(
                        res.success ? "success" : "error",
                        res.success
                            ? !is_post
                                ? "Response posted!"
                                : "Post created!"
                            : res.message,
                    );

                    if (res.success === true) {
                        if (!is_post) {
                            app.smooth_remove(
                                document.getElementById(`question:${question}`),
                                500,
                            );
                        }

                        return resolve(res);
                    } else {
                        return reject(res);
                    }
                });
        });
    });

    self.define("edit", function ({ $, app }, id, content) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/responses/${id}`, {
                method: "PUT",
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
                    } else {
                        return reject(res);
                    }
                });
        });
    });

    self.define("edit_tags", function ({ $, app }, id, tags) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/responses/${id}/tags`, {
                method: "PUT",
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
                    } else {
                        return reject(res);
                    }
                });
        });
    });

    self.define("delete", function ({ $, app }, id) {
        if (!confirm("Are you sure you want to do this?")) {
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
})();
