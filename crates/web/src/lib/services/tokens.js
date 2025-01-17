// @ts-nocheck
(() => {
    const self = reg_ns("tokens");

    // methods
    self.define("store_token", async function ({ $ }, token) {
        const tokens = $.tokens();
        tokens.push(token);
        window.location.setItem("tokens", JSON.stringify(tokens));
        return tokens;
    });

    self.define("remove_token", async function ({ $ }, token) {
        const tokens = $.tokens();
        const index = tokens.indexOf(token) || 0;
        tokens.splice(index, 1);
        window.location.setItem("tokens", JSON.stringify(tokens));
        return tokens;
    });

    self.define("tokens", async function (_) {
        return JSON.parse(window.location.getItem("tokens") || "[]");
    });

    // ui
})();
