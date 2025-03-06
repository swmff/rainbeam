(() => {
    const self = reg_ns("chats", ["app", "notifications"]);

    self.define("create", function ({ $, app }, id) {
        fetch(`/api/v1/chats/from_user/${id}`, {
            method: "POST",
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

    self.define("leave", async function ({ $, app }, id) {
        if (
            !(await trigger("app::confirm", [
                "Are you sure you want to do this?",
            ]))
        ) {
            return;
        }

        fetch(`/api/v1/chats/${id}`, {
            method: "DELETE",
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

    self.define(
        "msg.html",
        function ({ $, app, notifications }, msg, bind_to, is_own) {
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

                    // remove notification (if we have enough information)
                    if (window.CHAT_USER_ID && window.CHAT_ID && !is_own) {
                        notifications.delete(
                            `msg:${window.CHAT_USER_ID}:${window.CHAT_ID}`,
                        );
                    }
                });
        },
    );

    self.define("msg_delete", async function ({ $, app }, id) {
        if (
            !(await trigger("app::confirm", [
                "Are you sure you want to do this?",
            ]))
        ) {
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

    self.define("msg_edit", function ({ $, app }, id, content) {
        return new Promise((resolve, reject) => {
            fetch(`/api/v1/messages/${id}`, {
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
                        res.success ? "Message edited!" : res.message,
                    );

                    // update message on page
                    fetch("/api/v1/chats/_app/render", {
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
                            document.getElementById(
                                `msg_content:${id}`,
                            ).innerHTML = res;
                        });

                    globalThis.message_contents[id] = content;

                    // return
                    if (res.success) {
                        return resolve();
                    }

                    reject();
                });
        });
    });

    // ui
    self.define("ui::above_editor_text", function above_form_text(_, text) {
        if (text === "") {
            document.getElementById("above_form_text").style.display = "none";
            return;
        }

        document.getElementById("above_form_text").innerText = text;
        document.getElementById("above_form_text").style.display = "block";
    });

    self.define("ui::views.editor", function ({ $ }, id) {
        $["ui::above_editor_text"]("Editing message");
        $.EDIT_MESSAGE_ID = id;

        setTimeout(() => {
            globalThis.message_editor_.setValue(
                globalThis.message_contents[id],
            );
        }, 250);

        document.getElementById("message_writer_form").style.display = "none";
        document.getElementById("message_editor_form").style.display = "flex";

        document.getElementById("messages").style.opacity = "50%";
        document.getElementById("messages").style.userSelect = "none";
        document.getElementById("messages").style.pointerEvents = "none";
    });

    self.define("ui::views.writer", function ({ $ }, id) {
        $["ui::above_editor_text"]("");
        $.EDIT_MESSAGE_ID = "";

        document.getElementById("message_writer_form").style.display = "flex";
        document.getElementById("message_editor_form").style.display = "none";

        document.getElementById("messages").style.opacity = "100%";
        document.getElementById("messages").style.userSelect = "unset";
        document.getElementById("messages").style.pointerEvents = "unset";
    });
})();
