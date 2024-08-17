(() => {
    const app = reg_ns("app");

    app.define("fold_nav", function ({ $ }) {
        if (!$.nav_folded) {
            for (const nav of Array.from(document.querySelectorAll("nav"))) {
                nav.style.display = "none";
            }

            document.getElementById("folded_nav").style.display = "flex";
        } else {
            for (const nav of Array.from(document.querySelectorAll("nav"))) {
                nav.style.display = "flex";
            }

            document.getElementById("folded_nav").style.display = "none";
        }

        $.nav_folded = !($.nav_folded || false);
    });

    app.define("clean_date_codes", function ({ $ }) {
        for (const element of Array.from(document.querySelectorAll(".date"))) {
            if (isNaN(element.innerText)) {
                continue;
            }

            element.setAttribute(
                "title",
                new Date(parseInt(element.innerText)).toLocaleString(),
            );

            element.innerText = new Date(
                parseInt(element.innerText),
            ).toLocaleDateString();
        }
    });

    app.define("i_know_you", function (_) {
        // this will store your current username in localStorage
        fetch("/api/auth/me")
            .then((res) => res.json())
            .then((res) => {
                window.localStorage.setItem("me", res.message);
            });
    });

    app.define("do_i_know_you", function ({ $ }) {
        if (window.localStorage.getItem("me")) {
            globalThis.username = window.localStorage.getItem("me");
            return true;
        }

        return false;
    });

    app.define("logout", function (_) {
        if (!confirm("Are you sure you would like to do this?")) {
            return;
        }

        fetch("/api/auth/logout", { method: "POST" }).then(() => {
            window.location.href = "/";
        });
    });

    app.define("copy_text", function (_, text) {
        navigator.clipboard.writeText(text);
        alert("Copied!");
    });

    // hooks
    app.define("hook.dropdown", function (_, event) {
        let target = event.target;

        while (!target.matches(".dropdown")) {
            target = target.parentElement;
        }

        // close all others
        for (const dropdown of Array.from(
            document.querySelectorAll(".inner[open]"),
        )) {
            dropdown.removeAttribute("open");
        }

        // open
        setTimeout(() => {
            for (const dropdown of Array.from(
                target.querySelectorAll(".inner"),
            )) {
                dropdown.toggleAttribute("open");
            }
        }, 5);
    });

    app.define("hook.dropdown.init", function (_, bind_to) {
        bind_to.addEventListener("click", (event) => {
            if (
                event.target.matches(".dropdown") ||
                event.target.matches("[exclude=dropdown]")
            ) {
                return;
            }

            for (const dropdown of Array.from(
                document.querySelectorAll(".inner[open]"),
            )) {
                dropdown.removeAttribute("open");
            }
        });
    });

    app.define("hook.character_counter", function (_, event) {
        let target = event.target;

        while (!target.matches("textarea, input")) {
            target = target.parentElement;
        }

        const counter = document.getElementById(`${target.id}:counter`);
        counter.innerText = `${target.value.length}/${target.getAttribute("maxlength")}`;
    });

    app.define("hook.character_counter.init", function (_, event) {
        for (const element of Array.from(
            document.querySelectorAll("[hook=counter]") || [],
        )) {
            const counter = document.getElementById(`${element.id}:counter`);
            counter.innerText = `0/${element.getAttribute("maxlength")}`;
            element.addEventListener("keyup", (e) =>
                app["hook.character_counter"](e),
            );
        }
    });

    // adomonition
    app.define("shout", function (_, type, content) {
        if (document.getElementById("admonition")) {
            // there can only be one
            document.getElementById("admonition").remove();
        }

        const element = document.createElement("div");
        element.id = "admonition";
        element.classList.add(`markdown-alert-${type}`);
        element.innerHTML = content
            .replaceAll("<", "&lt")
            .replaceAll(">", "&gt;");

        if (document.querySelector("#admonition_zone")) {
            document.querySelector("#admonition_zone").prepend(element);
            return;
        }

        document.querySelector("article").prepend(element);
    });

    // shout from query params
    const search = new URLSearchParams(window.location.search);

    if (search.get("ANNC")) {
        // get defaults
        // we'll always use the value given in a query param over the page-set value
        const secret_type = search.get("ANNC_TYPE")
            ? search.get("ANNC_TYPE")
            : globalThis._app_base.annc.type;

        // ...
        app.shout(secret_type, search.get("ANNC"));
    }

    // link filter
    app.define("link_filter", function (_) {
        for (const anchor of Array.from(document.querySelectorAll("a"))) {
            const url = new URL(anchor.href);
            if (
                anchor.href.startsWith("/") ||
                anchor.href.startsWith("javascript:") ||
                url.origin === window.location.origin
            ) {
                continue;
            }

            anchor.addEventListener("click", (e) => {
                e.preventDefault();
                document.getElementById("link_filter_url").innerText =
                    anchor.href;
                document.getElementById("link_filter_continue").href =
                    anchor.href;
                document.getElementById("link_filter").showModal();
            });
        }
    });
})();
