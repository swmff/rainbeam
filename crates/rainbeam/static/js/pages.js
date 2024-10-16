(() => {
    const self = reg_ns("pages", ["app"]);

    self.define("create", async function ({ $, app }, slug, content) {
        await app.debounce("pages:create");
        return new Promise((resolve, reject) => {
            fetch("/api/v1/pages", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    slug,
                    content,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Page created!" : res.message,
                    );

                    if (res.success === true) {
                        window.location.href = `/+p/${res.payload.id}`;
                        return resolve(res);
                    }

                    return reject(res);
                });
        });
    });

    self.define("edit", function ({ $, app }, id, content) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/pages/${id}`, {
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
                        res.success ? "Page edited!" : res.message,
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

        fetch(`/api/v1/pages/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Page deleted!" : res.message,
                );

                window.location.href = "/pages";
            });
    });
})();
