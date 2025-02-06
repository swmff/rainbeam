(() => {
    const app = reg_ns("app");

    // env
    app.USE_TENNIS_LOADER = true;
    app.DEBOUNCE = [];
    app.OBSERVERS = [];

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

            element.style.display = "inline-block";
        }
    });

    app.define("me", async function (_) {
        globalThis.__user = await (await fetch("/api/v0/auth/me")).json();
    });

    app.define("logout", async function (_) {
        if (
            !(await trigger("app:confirm", [
                "Are you sure you would like to do this?",
            ]))
        ) {
            return;
        }

        fetch("/api/v0/auth/untag", { method: "POST" }).then(() => {
            fetch("/api/v0/auth/logout", { method: "POST" }).then(() => {
                window.location.href = "/";
            });
        });
    });

    app.define(
        "icon",
        async function ({ $ }, icon_name, classes = "", style = "") {
            if (!$.ICONS) {
                $.ICONS = {};
            }

            // use existing icon text
            if ($.ICONS[icon_name]) {
                const parser = new DOMParser().parseFromString(
                    $.ICONS[icon_name],
                    "text/xml",
                );

                const icon_element = parser.firstChild;
                icon_element.setAttribute("class", classes);
                icon_element.setAttribute("style", style);

                return icon_element;
            }

            // fetch icon
            const icon = await (
                await fetch(`/static/build/icons/${icon_name}.svg`)
            ).text();

            const parser = new DOMParser().parseFromString(icon, "text/xml");

            const icon_element = parser.firstChild;
            icon_element.setAttribute("class", classes);
            icon_element.setAttribute("style", style);

            return icon_element;
        },
    );

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

    app.define("intent_bluesky", function ({ $ }, text, link) {
        text += ` ${link}`;
        window.open(
            `https://bsky.app/intent/compose?text=${encodeURIComponent(text)}`,
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

    app.define("disconnect_observers", function ({ $ }) {
        for (const observer of $.OBSERVERS) {
            observer.disconnect();
        }

        $.OBSERVERS = [];
    });

    app.define(
        "offload_work_to_client_when_in_view",
        function (_, entry_callback) {
            // instead of spending the time on the server loading everything before
            // returning the page, we can instead of just create an IntersectionObserver
            // and send individual requests as we see the element it's needed for
            const seen = [];
            return new IntersectionObserver(
                (entries) => {
                    for (const entry of entries) {
                        const element = entry.target;
                        if (!entry.isIntersecting || seen.includes(element)) {
                            continue;
                        }

                        seen.push(element);
                        entry_callback(element);
                    }
                },
                {
                    root: document.body,
                    rootMargin: "0px",
                    threshold: 1.0,
                },
            );
        },
    );

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
            document.querySelectorAll(".inner.open"),
        )) {
            dropdown.classList.remove("open");
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
                dropdown.classList.add("open");

                if (dropdown.classList.contains("open")) {
                    dropdown.removeAttribute("aria-hidden");
                } else {
                    dropdown.setAttribute("aria-hidden", "true");
                }
            }
        }, 5);
    });

    app.define("hook.dropdown.init", function (_, bind_to) {
        for (const dropdown of Array.from(
            document.querySelectorAll(".inner"),
        )) {
            dropdown.setAttribute("aria-hidden", "true");
        }

        bind_to.addEventListener("click", (event) => {
            if (
                event.target.matches(".dropdown") ||
                event.target.matches("[exclude=dropdown]")
            ) {
                return;
            }

            for (const dropdown of Array.from(
                document.querySelectorAll(".inner.open"),
            )) {
                dropdown.classList.remove("open");
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
        function ({ $ }, partial, full, attach, wrapper, page, run_on_load) {
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

                            if (
                                text.length < 100 ||
                                text.includes('data-marker="no-results"')
                            ) {
                                // pretty much blank content, no more pages
                                wrapper.removeEventListener("scroll", event);

                                if (globalThis._app_base.ns_store.$questions) {
                                    trigger("questions:carp");
                                }

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

                            if (globalThis._app_base.ns_store.$questions) {
                                trigger("questions:carp");
                            }

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
                                if (document.getElementById("initial_loader")) {
                                    console.log("partial blocked");
                                    return;
                                }

                                page += 1;
                                await load_partial();
                                await $["hook.partial_embeds"]();

                                if (globalThis._app_base.ns_store.$questions) {
                                    trigger("questions:carp");
                                }
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
            paragraph.innerText = paragraph.innerText.replace(groups[0], "");
            paragraph.parentElement.innerHTML += `<include-partial
                src="/_app/components/response.html?id=${groups[2]}&do_render_nested=false"
                uses="app:clean_date_codes,app:link_filter,app:hook.alt"
            ></include-partial>`;
        }

        for (const paragraph of Array.from(
            document.querySelectorAll("span[class] p"),
        )) {
            const groups = /(\/inbox\/mail\/letter\/)([\w]+)/.exec(
                paragraph.innerText,
            );

            if (groups === null) {
                continue;
            }

            // add embed
            paragraph.innerText = paragraph.innerText.replace(groups[0], "");
            paragraph.parentElement.innerHTML = `<include-partial
                src="/inbox/mail/_app/components/mail.html?id=${groups[2]}&do_render_nested=false"
                uses="app:clean_date_codes,app:link_filter,app:hook.alt,app:hook.partial_embeds"
            ></include-partial>${paragraph.parentElement.innerHTML}`;
        }
    });

    app.define("hook.check_reactions", async function ({ $ }) {
        const observer = $.offload_work_to_client_when_in_view(
            async (element) => {
                const reaction = await (
                    await fetch(
                        `/api/v1/reactions/${element.getAttribute("hook-arg:id")}`,
                    )
                ).json();

                if (reaction.success) {
                    element.children[0].classList.add("filled");
                }
            },
        );

        for (const element of Array.from(
            document.querySelectorAll("[hook=check_reaction]") || [],
        )) {
            observer.observe(element);
        }

        $.OBSERVERS.push(observer);
    });

    app.define("hook.tabs:switch", function (_, tab) {
        // tab
        for (const element of Array.from(
            document.querySelectorAll("[data-tab]"),
        )) {
            element.classList.add("hidden");
        }

        document
            .querySelector(`[data-tab="${tab}"]`)
            .classList.remove("hidden");

        // button
        if (document.querySelector(`[data-tab-button="${tab}"]`)) {
            for (const element of Array.from(
                document.querySelectorAll("[data-tab-button]"),
            )) {
                element.classList.remove("active");
            }

            document
                .querySelector(`[data-tab-button="${tab}"]`)
                .classList.add("active");
        }
    });

    app.define("hook.tabs:check", function ({ $ }, hash) {
        if (!hash || !hash.startsWith("#/")) {
            return;
        }

        $["hook.tabs:switch"](hash.replace("#/", ""));
    });

    app.define("hook.tabs", function ({ $ }) {
        $["hook.tabs:check"](window.location.hash); // initial check
        window.addEventListener("hashchange", (event) =>
            $["hook.tabs:check"](new URL(event.newURL).hash),
        );
    });

    // web api replacements
    app.define("prompt", function (_, msg) {
        const dialog = document.getElementById("web_api_prompt");
        document.getElementById("web_api_prompt:msg").innerText = msg;

        return new Promise((resolve, _) => {
            globalThis.web_api_prompt_submit = (value) => {
                dialog.close();
                return resolve(value);
            };

            dialog.showModal();
        });
    });

    app.define("prompt_long", function (_, msg) {
        const dialog = document.getElementById("web_api_prompt_long");
        document.getElementById("web_api_prompt_long:msg").innerText = msg;

        return new Promise((resolve, _) => {
            globalThis.web_api_prompt_long_submit = (value) => {
                dialog.close();
                return resolve(value);
            };

            dialog.showModal();
        });
    });

    app.define("confirm", function (_, msg) {
        const dialog = document.getElementById("web_api_confirm");
        document.getElementById("web_api_confirm:msg").innerText = msg;

        return new Promise((resolve, _) => {
            globalThis.web_api_confirm_submit = (value) => {
                dialog.close();
                return resolve(value);
            };

            dialog.showModal();
        });
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
    const annc_event = () => {
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
    };

    globalThis.annc_event = annc_event;
    document.documentElement.addEventListener("turbo:load", annc_event);
    document.documentElement.addEventListener("turbo:submit-end", (event) => {
        // navigate to html url if returned
        if (
            event.detail.fetchResponse.response.headers.get("Content-Type") ===
            "text/html"
        ) {
            window.location.href = event.detail.fetchResponse.response.url;
        }

        annc_event();
    });

    // toast
    app.define("toast", function ({ $ }, type, content, time_until_remove = 5) {
        const element = document.createElement("div");
        element.id = "toast";
        element.classList.add(type);
        element.classList.add("toast");
        element.innerHTML = `<span>${content
            .replaceAll("<", "&lt")
            .replaceAll(">", "&gt;")}</span>`;

        document.getElementById("toast_zone").prepend(element);

        const timer = document.createElement("span");
        element.appendChild(timer);

        timer.innerText = time_until_remove;
        timer.classList.add("timer");

        // start timer
        setTimeout(() => {
            clearInterval(count_interval);
            $.smooth_remove(element, 500);
        }, time_until_remove * 1000);

        const count_interval = setInterval(() => {
            time_until_remove -= 1;
            timer.innerText = time_until_remove;
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
