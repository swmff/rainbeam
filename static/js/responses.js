(() => {
    const self = reg_ns("responses");

    self.define("delete", function (_, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/v1/responses/${id}`, {
                method: "DELETE",
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:shout", [
                        res.success ? "tip" : "caution",
                        res.message || "Response deleted!",
                    ]);

                    document
                        .getElementById(`response:${id}`)
                        .setAttribute("disabled", "fully");
                });
    });
})();
