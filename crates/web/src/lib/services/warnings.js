// @ts-nocheck
(() => {
    const self = reg_ns("warnings", []);

    const accepted_warnings = JSON.parse(
        window.localStorage.getItem("accepted_warnings") || "{}"
    );

    self.define(
        "open",
        async function ({ $ }, warning_id, warning_hash, warning_page = "") {
            // check localStorage for this warning_id
            if (accepted_warnings[warning_id] !== undefined) {
                // check hash
                if (accepted_warnings[warning_id] !== warning_hash) {
                    // hash is not the same, show dialog again
                    delete accepted_warnings[warning_id];
                } else {
                    // return
                    return;
                }
            }

            // open page
            if (warning_page !== "") {
                window.location.href = warning_page;
                return;
            }
        }
    );

    self.define(
        "accept",
        function ({ _ }, warning_id, warning_hash, redirect = "/") {
            accepted_warnings[warning_id] = warning_hash;

            window.localStorage.setItem(
                "accepted_warnings",
                JSON.stringify(accepted_warnings)
            );

            setTimeout(() => {
                window.location.href = redirect;
            }, 100);
        }
    );
})();
