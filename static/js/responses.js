(() => {
    const self = reg_ns("responses", ["app"]);

    self.define("create", function ({ $, app }, question, content) {
        return new Promise((resolve, reject) => {
            fetch("/api/v1/responses", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    question,
                    content,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Response posted!" : res.message,
                    );

                    if (res.success === true) {
                        app.smooth_remove(
                            document.getElementById(`question:${question}`),
                            500,
                        );

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
