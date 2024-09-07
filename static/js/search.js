(() => {
    const self = reg_ns("search", ["app"]);

    self.drivers = {
        responses: "/search/responses?q=",
        questions: "/search/questions?q=",
        users: "/search/users?q=",
        posts: "/search/posts?q=",
    };

    self.define("run", function ({ $, app }, driver, query) {
        const loc = $.drivers[driver];

        if (!loc) {
            return app.toast("error", "Invalid search driver");
        }

        window.location.href = `${loc}${query}`;
    });
})();
