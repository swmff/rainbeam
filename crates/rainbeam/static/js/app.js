(() => {
    const app = reg_ns("app");

    // env
    app.USE_TENNIS_LOADER = true;
    app.DEBOUNCE = [];

    // ...
    app.define("try_use", function (_, ns_name, callback) {
        // attempt to get existing namespace
        if (globalThis._app_base.ns_store[`$${ns_name}`]) {
            return callback(globalThis._app_base.ns_store[`$${ns_name}`]);
        }

        // otherwise, call normal use
        use(ns_name, callback);
    });

    app.define("debounce", function ({ $ }, name) {
        return new Promise((resolve, reject) => {
            if ($.DEBOUNCE.includes(name)) {
                return reject();
            }

            $.DEBOUNCE.push(name);

            setTimeout(() => {
                delete $.DEBOUNCE[$.DEBOUNCE.indexOf(name)];
            }, 1000);

            return resolve();
        });
    });

    app.define("rel_date", function (_, date) {
        // stolen and slightly modified because js dates suck
        const diff = (new Date().getTime() - date.getTime()) / 1000;
        const day_diff = Math.floor(diff / 86400);

        if (Number.isNaN(day_diff) || day_diff < 0 || day_diff >= 31) {
            return;
        }

        return (
            (day_diff === 0 &&
                ((diff < 60 && "just now") ||
                    (diff < 120 && "1 minute ago") ||
                    (diff < 3600 && Math.floor(diff / 60) + " minutes ago") ||
                    (diff < 7200 && "1 hour ago") ||
                    (diff < 86400 &&
                        Math.floor(diff / 3600) + " hours ago"))) ||
            (day_diff === 1 && "Yesterday") ||
            (day_diff < 7 && day_diff + " days ago") ||
            (day_diff < 31 && Math.ceil(day_diff / 7) + " weeks ago")
        );
    });

    app.define("clean_date_codes", function ({ $ }) {
        for (const element of Array.from(document.querySelectorAll(".date"))) {
            if (element.getAttribute("data-unix")) {
                // this allows us to run the function twice on the same page
                // without errors from already rendered dates
                element.innerText = element.getAttribute("data-unix");
            }

            element.setAttribute("data-unix", element.innerText);
            const then = new Date(Number.parseInt(element.innerText));

            if (Number.isNaN(element.innerText)) {
                continue;
            }

            element.setAttribute("title", then.toLocaleString());

            let pretty = $.rel_date(then);

            if (screen.width < 900 && pretty !== undefined) {
                // shorten dates even more for mobile
                pretty = pretty
                    .replaceAll(" minutes ago", "m")
                    .replaceAll(" minute ago", "m")
                    .replaceAll(" hours ago", "h")
                    .replaceAll(" hour ago", "h")
                    .replaceAll(" days ago", "d")
                    .replaceAll(" day ago", "d")
                    .replaceAll(" weeks ago", "w")
                    .replaceAll(" week ago", "w")
                    .replaceAll(" months ago", "m")
                    .replaceAll(" month ago", "m")
                    .replaceAll(" years ago", "y")
                    .replaceAll(" year ago", "y");
            }

            element.innerText =
                pretty === undefined ? then.toLocaleDateString() : pretty;
        }
    });

    app.define("logout", function (_) {
        if (!confirm("Are you sure you would like to do this?")) {
            return;
        }

        fetch("/api/v0/auth/untag", { method: "POST" }).then(() => {
            fetch("/api/v0/auth/logout", { method: "POST" }).then(() => {
                window.location.href = "/";
            });
        });
    });

    app.define("copy_text", function ({ $ }, text) {
        navigator.clipboard.writeText(text);
        $.toast("success", "Copied!");
    });

    app.define("intent_twitter", function ({ $ }, text, link) {
        window.open(
            `https://twitter.com/intent/tweet?text=${encodeURIComponent(text)}&url=${encodeURIComponent(link)}`,
        );

        $.toast("success", "Opened intent!");
    });

    app.define("smooth_remove", function (_, element, ms) {
        // run animation
        element.style.animation = `fadeout ease-in-out 1 ${ms}ms forwards running`;

        // remove
        setTimeout(() => {
            element.remove();
        }, ms);
    });

    app.define("skin:tennis_proc", function (_, css_source) {
        return new Promise((resolve, reject) => {
            app.try_use("tennis", async (tennis) => {
                // process
                const processed = await tennis.proc(css_source);

                // create blob
                const blob = new Blob([processed], { type: "text/css" });
                const url = URL.createObjectURL(blob);

                // return
                resolve(url);
            });
        });
    });

    app.define("skin", async function ({ $ }, skin) {
        if (skin === "sparkler") {
            regns_log("warn", `[app skin] skin is invalid, skipped: ${skin}`);

            // if (document.getElementById("skin_import")) {
            //     document.getElementById("skin_import").remove();
            // }

            return;
        }

        console.info(`[app skin] registered skin: ${skin}`);

        // add file extension and full path
        skin = `/static/skins/${skin}.css`;

        // preprocess css and load into blob
        if ($.USE_TENNIS_LOADER) {
            if (globalThis[`${skin}:blob`]) {
                // use existing blob from previous state,
                // this prevents the old theme flashing
                skin = globalThis[`${skin}:blob`];
            } else {
                // fetch
                const css_source = await (await fetch(skin)).text();
                const origin_skin = skin.toString();

                skin = await $["skin:tennis_proc"](css_source);
                globalThis[`${origin_skin}:blob`] = skin.toString();
            }
        }

        // ...
        if (document.getElementById("skin_import")) {
            document.getElementById("skin_import").innerHTML =
                `@import url("${skin}");`;
            return;
        }

        document.head.innerHTML += `<style id="skin_import">@import url("${skin}");</style>`;
    });

    app.define("load_skin", async function ({ $ }) {
        const skin = window.localStorage.getItem("skin");

        if (!skin) {
            return;
        }

        await $.skin(skin);
    });

    app.define("ban_ip", function (_, ip) {
        const reason = prompt(
            "Please explain your reason for banning this IP below:",
        );

        if (!reason) {
            return;
        }

        fetch("/api/v0/auth/ipbans", {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                ip,
                reason,
            }),
        })
            .then((res) => res.json())
            .then((res) => {
                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.message || "IP banned!",
                ]);
            });
    });

    app.define("check_app", function (_) {
        const is_app = window.__TAURI_INTERNALS__ !== undefined;

        if (is_app) {
            for (const element of Array.from(
                document.querySelectorAll(".app_only"),
            )) {
                element.classList.remove("app_only");
            }
        } else {
            return [false, {}];
        }

        return [true, window.__TAURI_INTERNALS__];
    });

    // hooks
    app.define("hook.scroll", function (_, scroll_element, track_element) {
        const goals = [150, 250, 500, 1000];

        track_element.setAttribute("data-scroll", "0");
        scroll_element.addEventListener("scroll", (e) => {
            track_element.setAttribute("data-scroll", scroll_element.scrollTop);

            for (const goal of goals) {
                const name = `data-scroll-${goal}`;
                if (scroll_element.scrollTop >= goal) {
                    track_element.setAttribute(name, "true");
                } else {
                    track_element.removeAttribute(name);
                }
            }
        });
    });

    app.define("hook.dropdown", function (_, event) {
        event.stopImmediatePropagation();
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
                // check y
                const box = target.getBoundingClientRect();

                let parent = dropdown.parentElement;

                while (!parent.matches("html, .window")) {
                    parent = parent.parentElement;
                }

                let parent_height = parent.getBoundingClientRect().y;

                if (parent.nodeName === "HTML") {
                    parent_height = window.screen.height;
                }

                const scroll = window.scrollY;
                const height = parent_height;
                const y = box.y + scroll;

                if (y > height - scroll - 300) {
                    dropdown.classList.add("top");
                } else {
                    dropdown.classList.remove("top");
                }

                // open
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

    app.define("hook.long", function (_, element, full_text) {
        element.classList.remove("hook:long.hidden_text");
        element.innerHTML = full_text;
    });

    app.define("hook.long_text.init", function (_, event) {
        for (const element of Array.from(
            document.querySelectorAll("[hook=long]") || [],
        )) {
            const is_long = element.innerText.length >= 64 * 16;

            if (!is_long) {
                continue;
            }

            element.classList.add("hook:long.hidden_text");

            if (element.getAttribute("hook-arg") === "lowered") {
                element.classList.add("hook:long.hidden_text+lowered");
            }

            const html = element.innerHTML;
            const short = html.slice(0, 64 * 16);
            element.innerHTML = `${short}...`;

            // event
            const listener = () => {
                app["hook.long"](element, html);
                element.removeEventListener("click", listener);
            };

            element.addEventListener("click", listener);
        }
    });

    app.define("hook.warning", function (_, event) {
        for (const element of Array.from(
            document.querySelectorAll("[data-warning]") || [],
        )) {
            const warning = element.getAttribute("data-warning");

            if (warning === "") {
                continue;
            }

            element.style.position = "relative";
            element.style.overflow = "hidden";

            const warning_element = document.createElement("div");
            warning_element.setAttribute(
                "style",
                `position: absolute;
                top: 0;
                left: 0;
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
                gap: 0.25rem;
                width: 100%;
                height: 100%;
                border-radius: inherit;
                cursor: pointer;
                padding: 1rem;
                background: var(--color-raised);`,
            );

            warning_element.innerHTML = `<p>${warning}</p><button class="primary bold round-lg">View content</button>`;
            element.appendChild(warning_element);

            // compute new height
            const warning_rect = warning_element.getBoundingClientRect();
            const paragraph_rect = warning_element
                .querySelector("p")
                .getBoundingClientRect();

            element.style.height = `${warning_rect.height + paragraph_rect.height}px`;

            // event
            const listener = () => {
                warning_element.removeEventListener("click", listener);
                warning_element.remove();

                element.style.height = "auto";
                element.style.overflow = "unset";
            };

            warning_element.addEventListener("click", listener);
        }
    });

    app.define("hook.alt", function (_) {
        for (const element of Array.from(
            document.querySelectorAll("img") || [],
        )) {
            if (element.getAttribute("alt") && !element.getAttribute("title")) {
                element.setAttribute("title", element.getAttribute("alt"));
            }
        }
    });

    app.define("hook.ips", function ({ $ }) {
        for (const anchor of Array.from(document.querySelectorAll("a"))) {
            try {
                const href = new URL(anchor.href);

                if (href.pathname.startsWith("/+i/")) {
                    // IP expander
                    anchor.addEventListener("click", (e) => {
                        e.preventDefault();

                        if (
                            confirm(
                                'Would you like to ban this IP? Please press "Cancel" to open the first profile found with this IP instead of banning it.',
                            )
                        ) {
                            $.ban_ip(href.pathname.replace("/+i/", ""));
                        } else {
                            window.open(href.href, "_blank");
                        }
                    });
                }
            } catch {}
        }
    });

    app.define(
        "hook.attach_to_partial",
        function ({ $ }, partial, full, attach, wrapper, page) {
            return new Promise((resolve, reject) => {
                async function load_partial() {
                    const url = `${partial}?page=${page}`;
                    history.replaceState(
                        history.state,
                        "",
                        url.replace(partial, full),
                    );

                    fetch(url)
                        .then(async (res) => {
                            const text = await res.text();

                            if (text.length < 100) {
                                // pretty much blank content, no more pages
                                wrapper.removeEventListener("scroll", event);
                                return resolve();
                            }

                            attach.innerHTML += text;

                            $.clean_date_codes();
                            $.link_filter();
                            $["hook.alt"]();

                            if (globalThis._app_base.ns_store.$questions) {
                                trigger("questions:carp");
                            }
                        })
                        .catch(() => {
                            // done scrolling, no more pages (http error)
                            wrapper.removeEventListener("scroll", event);
                            resolve();
                        });
                }

                const event = async () => {
                    if (
                        wrapper.scrollTop + wrapper.offsetHeight + 100 >
                        attach.offsetHeight
                    ) {
                        app.debounce("app:partials")
                            .then(async () => {
                                page += 1;
                                await load_partial();
                            })
                            .catch(() => {
                                console.log("partial stuck");
                            });
                    }
                };

                wrapper.addEventListener("scroll", event);
            });
        },
    );

    app.define("hook.partial_embeds", function (_) {
        for (const paragraph of Array.from(
            document.querySelectorAll("span[class] p"),
        )) {
            const groups = /(\/\+r\/)([\w]+)/.exec(paragraph.innerText);

            if (groups === null) {
                continue;
            }

            // add embed
            paragraph.parentElement.innerHTML += `<include-partial
                src="/_app/components/response.html?id=${groups[2]}&do_render_nested=false"
                uses="app:clean_date_codes,app:link_filter,app:hook.alt"
            ></include-partial>`;
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

    // toast
    app.define("toast", function ({ $ }, type, content) {
        let time_until_remove = 5; // seconds

        const element = document.createElement("div");
        element.id = "toast";
        element.classList.add(type);
        element.classList.add("toast");
        element.innerHTML = content
            .replaceAll("<", "&lt")
            .replaceAll(">", "&gt;");

        document.getElementById("toast_zone").prepend(element);

        const timer = document.createElement("span");
        element.appendChild(timer);

        timer.innerText = `(${time_until_remove})`;
        timer.classList.add("timer");

        // start timer
        setTimeout(() => {
            clearInterval(count_interval);
            $.smooth_remove(element, 500);
        }, time_until_remove * 1000);

        const count_interval = setInterval(() => {
            time_until_remove -= 1;
            timer.innerText = `(${time_until_remove})`;
        }, 1000);
    });

    // link filter
    app.define("link_filter", function (_) {
        for (const anchor of Array.from(document.querySelectorAll("a"))) {
            if (anchor.href.length === 0) {
                continue;
            }

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
