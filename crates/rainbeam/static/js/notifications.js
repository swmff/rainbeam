(() => {
    const self = reg_ns("notifications", ["app"]);

    self.define("delete", async function ({ $, app }, id, conf) {
        // if (!conf) {
        //     if (!confirm("Are you sure you want to do this?")) {
        //         return;
        //     }
        // }

        fetch(`/api/v0/auth/notifications/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                if (document.getElementById(`notif:${id}`)) {
                    trigger("app::toast", [
                        res.success ? "success" : "error",
                        res.success ? "Notification deleted!" : res.message,
                    ]);

                    app.smooth_remove(
                        document.getElementById(`notif:${id}`),
                        500,
                    );
                }
            });
    });

    self.define("clear", async function (_, conf) {
        // if (!conf) {
        //     if (!confirm("Are you sure you want to do this?")) {
        //         return;
        //     }
        // }

        fetch("/api/v0/auth/notifications/clear", {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app::toast", [
                    res.success ? "success" : "error",
                    res.success ? "Notifications cleared!" : res.message,
                ]);
            });
    });

    self.define("onopen", function ({ $ }, id) {
        if (window.localStorage.getItem("clear_notifs") === "true") {
            $.delete(id, true);
        }
    });
})();
