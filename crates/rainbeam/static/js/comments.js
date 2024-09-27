(() => {
    const self = reg_ns("comments", ["app"]);

    self.define("create", function ({ $, app }, response, content, reply) {
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
})();