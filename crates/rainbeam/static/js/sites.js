(() => {
    const self = reg_ns("sites", ["app"]);

    self.define("create", async function ({ $, app }, slug, content) {
        await app.debounce("sites:create");
        return new Promise((resolve, reject) => {
            fetch("/api/v1/sites", {
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
                        res.success ? "Site created!" : res.message,
                    );

                    if (res.success === true) {
                        window.location.href = `/+s/${res.payload.id}`;
                        return resolve(res);
                    }

                    return reject(res);
                });
        });
    });

    self.define("edit", function ({ $, app }, id, content) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/sites/${id}`, {
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
                        res.success ? "Site edited!" : res.message,
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

        fetch(`/api/v1/sites/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Site deleted!" : res.message,
                );

                window.location.href = "/sites";
            });
    });

    self.define("render", function (_, content) {
        return new Promise((resolve, _) => {
            fetch("/api/v1/sites/_app/render", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    content,
                }),
            })
                .then((res) => res.text())
                .then((res) => {
                    resolve(res);
                });
        });
    });
})();
