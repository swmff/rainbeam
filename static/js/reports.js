(() => {
    const self = reg_ns("reports");

    self.define("bootstrap", function ({ $ }, type, target) {
        $.type = type;
        $.target = target;
        document.getElementById("report_dialog").showModal();
    });

    self.define("file", function ({ $ }, e) {
        e.preventDefault();
        fetch(`/api/v1/${$.type}/${$.target}/report`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                content: e.target.content.value,
                token: e.target.querySelector(".h-captcha textarea").value,
            }),
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:shout", [
                    res.success ? "tip" : "caution",
                    res.message || "Report filed!",
                ]);

                e.target.reset();
                document.getElementById("report_dialog").close();
            });
    });
})();
