(() => {
    const self = reg_ns("me");

    self.LOGIN_ACCOUNT_TOKENS = JSON.parse(
        window.localStorage.getItem("login_account_tokens") || "{}",
    );

    self.define("me", async function (_) {
        globalThis.__user = (
            await (await fetch("/api/v0/auth/me")).json()
        ).payload;

        return globalThis.__user;
    });

    self.define("logout", async function ({ $ }) {
        if (
            !(await trigger("app::confirm", [
                "Are you sure you would like to do this?",
            ]))
        ) {
            return;
        }

        // whoami?
        const me = await $.me();

        // remove token
        const tokens = $.LOGIN_ACCOUNT_TOKENS;

        if (tokens[me.username]) {
            delete tokens[me.username];
            $.set_login_account_tokens(tokens);
        }

        // ...
        fetch("/api/v0/auth/untag", { method: "POST" }).then(() => {
            fetch("/api/v0/auth/logout", { method: "POST" }).then(() => {
                // get the first saved token and login as that
                const first = Object.keys(tokens)[0];

                if (first) {
                    $.login(first);
                    return;
                }

                window.location.href = "/";
            });
        });
    });

    self.define(
        "set_login_account_tokens",
        ({ $ }, value) => {
            $.LOGIN_ACCOUNT_TOKENS = value;
            window.localStorage.setItem(
                "login_account_tokens",
                JSON.stringify(value),
            );
        },
        ["object"],
    );

    self.define("login", function ({ $ }, username) {
        const token = self.LOGIN_ACCOUNT_TOKENS[username];

        if (!token) {
            return;
        }

        window.location.href = `/api/v0/auth/callback?token=${token}`;
    });

    self.define("ui::render", function ({ $ }, element) {
        element.innerHTML = "";
        for (const token of Object.entries($.LOGIN_ACCOUNT_TOKENS)) {
            element.innerHTML += `<button class="card w-full justify-start" onclick="trigger('me::login', ['${token[0]}'])">
                <img
                    title="${token[0]}'s avatar"
                    src="/api/v0/auth/profile/${token[0]}/avatar"
                    alt=""
                    class="avatar"
                    style="--size: 24px"
                />

                <span>${token[0]}</span>
            </button>`;
        }
    });
})();
