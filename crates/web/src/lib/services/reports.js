// @ts-nocheck
(() => {
    const self = reg_ns("reports");

    self.define("fill", function ({ $ }, type, target) {
        $.type = type;
        $.target = target;
    });

    self.define("bootstrap", function (_, type, target) {
        window.open(`/intents/report?type=${type}&target=${target}`);
    });

    self.define("file", function ({ $ }, e) {
        e.preventDefault();
        fetch(`/api/v1/${$.type}/${$.target}/report`, {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                content: e.target.content.value,
                token: e.target.querySelector(".h-captcha textarea").value
            })
        })
            .then((res) => res.json())
            .then((res) => {
                if (res.success === true) {
                    alert(res.message);
                    window.close();
                    return;
                }

                trigger("app:shout", ["caution", res.message]);
                e.target.reset();
            });
    });
})();
