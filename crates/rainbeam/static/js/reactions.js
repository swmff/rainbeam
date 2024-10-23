(() => {
    const self = reg_ns("reactions");

    self.define("create", function (_, id, type) {
        fetch(`/api/v1/reactions/${id}`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                type,
            }),
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.message || "Reaction added!",
                ]);
            });
    });

    self.define("delete", function (_, id) {
        fetch(`/api/v1/reactions/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.message || "Reaction removed!",
                ]);
            });
    });

    self.define("has-reacted", function (_, id) {
        return new Promise((resolve, _) => {
            fetch(`/api/v1/reactions/${id}`, {
                method: "GET",
            })
                .then((res) => res.json())
                .then((res) => {
                    return resolve(res.success);
                });
        });
    });

    self.define("toggle", async function ({ $ }, id, type) {
        const remove = (await $["has-reacted"](id)) == true;

        if (remove) {
            return $.delete(id);
        }

        return $.create(id, type);
    });
})();
