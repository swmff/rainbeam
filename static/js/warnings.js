(() => {
    const self = reg_ns("warnings", ["dialogs"]);

    self.define(
        "open",
        async function ({ $, dialogs }, warning_id, warning_hash) {
            // check localStorage for this warning_id
            const accepted_warnings = JSON.parse(
                window.localStorage.getItem("accepted_warnings") || "{}",
            );

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

            // open dialog
            $.warning = dialogs.get("warning_dialog");
            $.warning.open();

            dialogs["event:confirm"]("warning_dialog", () => {
                // write accepted_warnings
                accepted_warnings[warning_id] = warning_hash;

                window.localStorage.setItem(
                    "accepted_warnings",
                    JSON.stringify(accepted_warnings),
                );
            });
        },
    );

    self.define("accept", function ({ $, dialogs }) {
        $.warning.close();
    });
})();
