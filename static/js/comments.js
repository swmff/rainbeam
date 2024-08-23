(() => {
    const self = reg_ns("comments");

    self.define("delete", function (_, id) {
        if (!confirm("Are you sure you want to do this?")) {
            return;
        }

        fetch(`/api/v1/comments/${id}`, {
            method: "DELETE",
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.success ? "Comment deleted!" : res.message,
                ]);

                document
                    .getElementById(`comment:${id}`)
                    .setAttribute("disabled", "fully");
            });
    });
})();
