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

    self.define("name", function ({ $, app }, id, name) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/chats/${id}/name`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    chat: id,
                    name,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Name updated!" : res.message,
                    );

                    if (res.success === true) {
                        resolve();
                    } else {
                        reject();
                    }
                });
        });
    });

    self.define("add", function ({ $, app }, id, friend) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/chats/${id}/add`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    chat: id,
                    friend,
                }),
            })
                .then((res) => res.json())
                .then((res) => {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Friend added to chat!" : res.message,
                    );

                    if (res.success === true) {
                        resolve();
                    } else {
                        reject();
                    }
                });
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
                        resolve(res.payload);
                    } else {
                        reject();
                    }
                });
        });
    });

    self.define("msg.html", function ({ $, app }, msg, bind_to) {
        fetch("/chats/_app/msg.html", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify(msg),
        })
            .then((res) => res.text())
            .then((html) => {
                console.info("msg added:", msg[0].id);

                const element = document.createElement("div");
                element.innerHTML = html;
                element.style.display = "contents";

                bind_to.prepend(element);
                app.clean_date_codes();
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
