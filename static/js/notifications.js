(() => {
    const self = reg_ns("notifications");

    self.define("delete", function (_, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/auth/notifications/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:shout", [
                    res.success ? "tip" : "caution",
                    res.message || "Notification deleted!",
                ]);

                document
                    .getElementById(`notif:${id}`)
                    .setAttribute("disabled", "fully");
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
                trigger("app:shout", [
                    res.success ? "tip" : "caution",
                    res.message || "Notifications cleared!",
                ]);
            });
    });
})();
