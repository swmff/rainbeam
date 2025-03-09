(() => {
    const self = reg_ns("questions", ["app"]);

    self.define(
        "create",
        async function ({ $, app }, recipient, content, anonymous, media = "") {
            await app.debounce("responses::create");
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
                        media: media || [],
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
                        }

                        return reject(res);
                    });
            });
        },
    );

    self.define("delete", async function ({ $, app }, id, do_confirm = true) {
        if (do_confirm) {
            if (
                !(await trigger("app::confirm", [
                    "Are you sure you want to do this?",
                ]))
            ) {
                return;
            }
        }

        fetch(`/api/v1/questions/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                if (do_confirm) {
                    app.toast(
                        res.success ? "success" : "error",
                        res.success ? "Question deleted!" : res.message,
                    );

                    app.smooth_remove(
                        document.getElementById(`question:${id}`),
                        500,
                    );
                }
            });
    });

    self.define("carp", function () {
        use("carp", (carp) => {
            for (const question of document.querySelectorAll(
                ".question_content",
            )) {
                const p = question.querySelector("p");

                if (!p || !p.innerText.startsWith("--CARP")) {
                    continue;
                }

                if (question.querySelector("canvas")) {
                    // remove existing
                    question.querySelector("canvas").remove();
                }

                const gerald = carp.new(question, true);
                gerald.create_canvas();
                gerald.from_string(p.innerText.replace("--CARP", ""));
            }
        });
    });

    self.define("ipblock", async function ({ $, app }, id) {
        if (
            !(await trigger("app::confirm", [
                "Are you sure you want to do this?",
            ]))
        ) {
            return;
        }

        fetch(`/api/v1/questions/${id}/ipblock`, {
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
