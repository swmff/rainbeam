(() => {
    const self = reg_ns("questions");

    self.define("delete", function (_, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/v1/questions/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:shout", [
                    res.success ? "tip" : "caution",
                    res.message || "Question deleted!",
                ]);

                document
                    .getElementById(`question:${id}`)
                    .setAttribute("disabled", "fully");
            });
    });
})();
