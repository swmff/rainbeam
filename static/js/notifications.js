(() => {
    const self = reg_ns("notifications");

    self.define("delete", function ({ $, app }, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/auth/notifications/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.success ? "Notification deleted!" : res.message,
                ]);

                app.smooth_remove(document.getElementById(`notif:${id}`), 500);
            });
    });

    self.define("clear", function (_, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/auth/notifications/clear`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.success ? "Notifications cleared!" : res.message,
                ]);
            });
    });
})();
