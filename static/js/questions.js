(() => {
    const self = reg_ns("questions", ["app"]);

    self.define("create", function ({ $, app }, recipient, content, anonymous) {
        return new Promise((resolve, reject) => {
            fetch("/api/v1/questions", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    recipient,
                    content,
                    anonymous,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Question asked!" : res.message,
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

        fetch(`/api/v1/questions/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Question deleted!" : res.message,
                );

                app.smooth_remove(
                    document.getElementById(`question:${id}`),
                    500,
                );
            });
    });
})();
