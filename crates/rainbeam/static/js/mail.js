(() => {
    const self = reg_ns("mail", ["app"]);

    self.define("create", function ({ $, app }, recipient, title, content) {
        return new Promise((resolve, reject) => {
            fetch("/api/v0/auth/mail", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    recipient: recipient.trim().split(","),
                    title,
                    content,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Mail sent!" : res.message,
                    );

                    if (res.success === true) {
                        resolve(res.payload);
                    } else {
                        reject();
                    }
                });
        });
    });

    self.define("delete", async function ({ $, app }, id) {
        if (
            !(await trigger("app::confirm", [
                "Are you sure you want to do this?",
            ]))
        ) {
            return;
        }

        fetch(`/api/v0/auth/mail/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Mail deleted!" : res.message,
                );

                app.smooth_remove(document.getElementById(`mail:${id}`), 500);
            });
    });

    self.define("state", async function ({ $, app }, id, state) {
        fetch(`/api/v0/auth/mail/${id}/state`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                state,
            }),
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Mail updated!" : res.message,
                );
            });
    });
})();
