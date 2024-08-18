(() => {
    const self = reg_ns("reactions");

    self.define("create", function (_, id) {
        fetch(`/api/v1/reactions/${id}`, {
            method: "POST",
        })
            .then((res) => res.json())
            .then((res) => {
                // trigger("app:shout", [
                //     res.success ? "tip" : "caution",
                //     res.message || "Reaction added!",
                // ]);

                alert(res.message || "Reaction added!");
                window.close();
            });
    });

    self.define("delete", function (_, id) {
        fetch(`/api/v1/reactions/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                // trigger("app:shout", [
                //     res.success ? "tip" : "caution",
                //     res.message || "Reaction removed!",
                // ]);

                alert(res.message || "Reaction removed!");
                window.close();
            });
    });
})();
