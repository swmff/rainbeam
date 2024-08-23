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
                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.success ? "Question deleted!" : res.message,
                ]);

                document
                    .getElementById(`question:${id}`)
                    .setAttribute("disabled", "fully");
            });
    });
})();
