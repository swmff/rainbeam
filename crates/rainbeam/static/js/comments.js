(() => {
    const self = reg_ns("comments", ["app"]);

    self.define(
        "create",
        async function ({ $, app }, response, content, reply, anonymous) {
            await app.debounce("responses:create");
            return new Promise((resolve, reject) => {
                fetch("/api/v1/comments", {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json",
                    },
                    body: JSON.stringify({
                        response,
                        content,
                        reply,
                        anonymous,
                    }),
                })
                    .then((res) => res.json())
                    .then((res) => {
                        app.toast(
                            res.success ? "success" : "error",
                            res.success ? "Comment posted!" : res.message,
                        );

                        if (res.success === true) {
                            return resolve(res);
                        }

                        return reject(res);
                    });
            });
        },
    );

    self.define("edit", function ({ $, app }, id, content) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/comments/${id}`, {
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
                        res.success ? "Comment edited!" : res.message,
                    );

                    if (res.success === true) {
                        return resolve(res);
                    }

                    return reject(res);
                });
        });
    });

    self.define("delete", function ({ $, app }, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/v1/comments/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Comment deleted!" : res.message,
                );

                app.smooth_remove(
                    document.getElementById(`comment:${id}`),
                    500,
                );
            });
    });

    self.define("ipblock", function ({ $, app }, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/v1/comments/${id}/ipblock`, {
            method: "POST",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "IP blocked!" : res.message,
                );
            });
    });
})();
