(() => {
    const self = reg_ns("chats", ["app"]);

    self.define("create", function ({ $, app }, id) {
        fetch(`/api/v1/chats/from_user/${id}`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Chat created!" : res.message,
                );

                if (res.success === true) {
                    window.location.href = `/chats/${res.payload.id}`;
                }
            });
    });

    self.define("leave", function ({ $, app }, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/v1/chats/${id}`, {
            method: "DELETE",
            headers: {
                "Content-Type": "application/json",
            },
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Chat left!" : res.message,
                );

                if (res.success === true) {
                    window.location.href = "/chats";
                }
            });
    });

    self.define("msg", function ({ $, app }, id, content) {
        return new Promise((resolve, reject) => {
            fetch("/api/v1/messages", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    chat: id,
                    content,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Message sent!" : res.message,
                    );

                    if (res.success === true) {
                        resolve();
                    } else {
                        reject();
                    }
                });
        });
    });

    self.define("msg_delete", function ({ $, app }, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/v1/messages/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                app.toast(
                    res.success ? "success" : "error",
                    res.success ? "Message deleted!" : res.message,
                );

                app.smooth_remove(
                    document.getElementById(`message:${id}`),
                    500,
                );
            });
    });
})();
