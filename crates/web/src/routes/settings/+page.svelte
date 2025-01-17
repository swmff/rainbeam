<script lang="ts">
    import { onMount } from "svelte";

    import { active_page } from "$lib/stores";
    import LangPicker from "$lib/components/LangPicker.svelte";
    import { Ellipsis, X } from "lucide-svelte";
    import { Option } from "$lib/classes/Option";
    active_page.set("settings.account");

    const { data } = $props();
    const lang = data.lang;
    const page = data.data;
    const user = Option.from(data.user).unwrap();

    const { ipblocks, relationships } = page;

    onMount(() => {
        (globalThis as any).update_theme = (theme: string) => {
            window.localStorage.setItem("theme", theme);
            document.documentElement.setAttribute("class", theme);
        };

        (globalThis as any).update_theme_preference = (pref: string) => {
            window.localStorage.setItem("theme-pref", pref);
        };

        (globalThis as any).update_css_preference = (pref: string) => {
            window.localStorage.setItem("css-pref", pref);
        };

        // fill current theme
        const current = window.localStorage.getItem("theme") || "";

        if (document.getElementById(current)) {
            (document.getElementById(current) as HTMLElement).setAttribute("selected", "true");
        }

        // fill current theme preference
        const pref = window.localStorage.getItem("theme-pref") || "";

        if (document.getElementById(pref)) {
            (document.getElementById(pref) as HTMLElement).setAttribute("selected", "true");
        }

        // fill current css preference
        const css_pref = window.localStorage.getItem("css-pref") || "";

        if (document.getElementById(css_pref)) {
            (document.getElementById(`css-${css_pref}`) as HTMLElement).setAttribute("selected", "true");
        }

        // fill extras
        if (window.localStorage.getItem("clear_notifs") === "true") {
            (document.getElementById("sparkler:clear_notifs") as HTMLElement).setAttribute("checked", "true");
        }

        if (window.localStorage.getItem("clear_all_notifs") === "true") {
            (document.getElementById("sparkler:clear_all_notifs") as HTMLElement).setAttribute("checked", "true");
        }

        // change username
        (document.getElementById("change_username") as HTMLElement).addEventListener("submit", (e) => {
            e.preventDefault();
            fetch(`/api/v0/auth/profile/${user.id}/username`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    password: (e.target as any).current_password_username.value,
                    new_name: (e.target as any).new_name.value
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:shout", [res.success ? "tip" : "caution", res.message || "Username changed!"]);

                    window.location.href = "#top";
                    (e.target as any).reset();
                });
        });

        // change password
        (document.getElementById("change_password") as HTMLElement).addEventListener("submit", (e) => {
            e.preventDefault();
            fetch(`/api/v0/auth/profile/${user.id}/password`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    password: (e.target as any).current_password.value,
                    new_password: (e.target as any).new_password.value
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:shout", [res.success ? "tip" : "caution", res.message || "Password changed!"]);

                    window.location.href = "#top";
                    (e.target as any).reset();
                });
        });

        // delete account
        (document.getElementById("delete_account") as HTMLElement).addEventListener("submit", async (e) => {
            e.preventDefault();

            if (!(await trigger("app:confirm", ["Are you 100% sure you want to do this?"]))) {
                return;
            }

            fetch("/api/v0/auth/me/delete", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    password: (e.target as any).current_password_delete.value
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:shout", [res.success ? "tip" : "caution", res.message || "Profile deleted, goodbye!"]);

                    window.location.href = "#top";
                    window.localStorage.removeItem("me");
                    (e.target as any).reset();
                });
        });

        setTimeout(async () => {
            if (window.location.search === "?note") {
                (document.getElementById("set_note_dialog") as any).showModal();
            } else if (window.location.search === "?note_clear") {
                if (!(await trigger("app:confirm", ["Are you sure you would like to clear your current status?"]))) {
                    window.close();
                    return;
                }

                // clear values
                (globalThis as any).update_kv("sparkler:status_note", "");
                (globalThis as any).update_kv("sparkler:status_emoji", "");

                // save
                (globalThis as any).save_settings().then(() => window.close());
            } else if (window.location.search === "?signature") {
                (document.getElementById("set_signature_dialog") as any).showModal();
            }
        }, 250);

        (globalThis as any).block_dialog = function () {
            // show confirmation
            (document.getElementById("block_dialog") as any).showModal();
        };

        (globalThis as any).block = function () {
            const username = (document.getElementById("sparkler:block_somebody") as any).value;

            fetch(`/api/v0/auth/relationships/block/${username}`, {
                method: "POST"
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [
                        res.success ? "success" : "error",
                        res.success ? "User blocked!" : res.message
                    ]);

                    window.close();
                });
        };

        (globalThis as any).remove_relationship = async function (username: string) {
            if (!(await trigger("app:confirm", ["Are you sure you want to do this?"]))) {
                return;
            }

            fetch(`/api/v0/auth/relationships/current/${username}`, {
                method: "DELETE"
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [
                        res.success ? "success" : "error",
                        res.success ? "Relationship removed!" : res.message
                    ]);

                    window.close();
                });
        };

        (globalThis as any).remove_ipblock = async function (id: string) {
            if (!(await trigger("app:confirm", ["Are you sure you want to do this?"]))) {
                return;
            }

            fetch(`/api/v0/auth/ipblocks/${id}`, {
                method: "DELETE"
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [
                        res.success ? "success" : "error",
                        res.success ? "IP block removed!" : res.message
                    ]);

                    window.close();
                });
        };

        // fill block_somebody
        const search = new URLSearchParams(window.location.search);

        if (search.get("block")) {
            setTimeout(() => {
                (document.getElementById("sparkler:block_somebody") as any).value = search.get("block");

                (globalThis as any).block_dialog();
            }, 100);
        }
    });
</script>

<div class="flex flex-col gap-4">
    <h4 class="title">{lang["settings:account.html:title.language"]}</h4>

    <LangPicker lang={data.lang_name} />

    <h4 class="title">
        {lang["settings:account.html:title.local_theming"]}
    </h4>

    <div class="flex flex-col gap-1">
        <label for="sparkler:website_theme">{lang["settings:account.html:label.website_theme"]}</label>

        <select
            name="sparkler:website_theme"
            id="sparkler:website_theme"
            onchange={(event) => {
                (globalThis as any).update_theme(
                    (event.target as any).options[(event.target as any).selectedIndex].value
                );
            }}
        >
            <option value="light" id="light">
                {lang["settings:text.light"]}
            </option>
            <option value="dark" id="dark">Dark</option>
            <option value="dark dim" id="dark dim">
                {lang["settings:text.dim"]}
            </option>
        </select>

        <p class="fade">
            This is just your local preferred theme! Profiles are always in light theme, but this will show on every
            other page.
        </p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:allow_profile_themes">{lang["settings:account.html:label.allow_profile_themes"]}</label>

        <select
            name="sparkler:allow_profile_themes"
            id="sparkler:allow_profile_themes"
            onchange={(event) => {
                (globalThis as any).update_theme_preference(
                    (event.target as any).options[(event.target as any).selectedIndex].value
                );
            }}
        >
            <option value="yes" id="yes">
                {lang["general:dialog.yes"]}
            </option>

            <option value="no" id="no">{lang["general:dialog.no"]}</option>
        </select>

        <p class="fade">This is a local perference! Changing this to "No" will not show custom profile themes.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:allow_profile_css">{lang["settings:account.html:label.allow_profile_css"]}</label>

        <select
            name="sparkler:allow_profile_css"
            id="sparkler:allow_profile_css"
            onchange={(event) => {
                (globalThis as any).update_css_preference(
                    (event.target as any).options[(event.target as any).selectedIndex].value
                );
            }}
        >
            <option value="yes" id="css-yes">
                {lang["general:dialog.yes"]}
            </option>

            <option value="no" id="css-no">
                {lang["general:dialog.no"]}
            </option>
        </select>

        <p class="fade">
            This is a local perference! Changing this to "No" will not show custom profile CSS. Applied colors will
            still be rendered.
        </p>
    </div>

    <h4 class="title">
        {lang["settings:account.html:title.local_behaviour"]}
    </h4>
    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:clear_notifs"
                id="sparkler:clear_notifs"
                onchange={(event) => {
                    window.localStorage.setItem("clear_notifs", (event.target as any).checked.toString());
                }}
            />

            <label for="sparkler:clear_notifs" class="normal">
                {lang["settings:account.html:label.clear_notifs"]}
            </label>
        </div>

        <p class="fade subtext">Clear specific notifications automatically whenever you open them.</p>
    </div>

    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:always_anon"
                id="sparkler:always_anon"
                onchange={(event) => {
                    window.localStorage.setItem("always_anon", (event.target as any).checked.toString());
                }}
            />

            <label for="sparkler:always_anon" class="normal">
                {lang["settings:account.html:label.always_anon"]}
            </label>
        </div>

        <p class="fade subtext">"Hide your name" will be checked by default.</p>
    </div>

    <h4 class="title">
        {lang["settings:account.html:title.profile_controls"]}
    </h4>
    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:limited_friend_requests"
                id="sparkler:limited_friend_requests"
                onchange={(event) => {
                    (globalThis as any).update_kv(
                        "sparkler:limited_friend_requests",
                        (event.target as any).checked.toString()
                    );
                }}
            />

            <label for="sparkler:limited_friend_requests" class="normal">
                {lang["settings:account.html:label.limited_friend_requests"]}
            </label>
        </div>
    </div>

    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:private_profile"
                id="sparkler:private_profile"
                onchange={(event) => {
                    (globalThis as any).update_kv("sparkler:private_profile", (event.target as any).checked.toString());
                }}
            />

            <label for="sparkler:private_profile" class="normal">
                {lang["settings:account.html:label.private_profile"]}
            </label>
        </div>

        <p class="fade subtext">Only allow friends to view your posts and feed.</p>
    </div>

    <div class="checkbox_container">
        <input
            type="checkbox"
            name="rainbeam:nsfw_profile"
            id="rainbeam:nsfw_profile"
            onchange={(event) => {
                (globalThis as any).update_kv("rainbeam:nsfw_profile", (event.target as any).checked.toString());
            }}
        />

        <label for="rainbeam:nsfw_profile" class="normal">
            {lang["settings:account.html:label.nsfw_profile"]}
        </label>
    </div>

    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:limited_chats"
                id="sparkler:limited_chats"
                onchange={(event) => {
                    (globalThis as any).update_kv("sparkler:limited_chats", (event.target as any).checked.toString());
                }}
            />

            <label for="sparkler:limited_chats" class="normal">
                {lang["settings:account.html:label.limited_chats"]}
            </label>
        </div>
    </div>

    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:allow_drawings"
                id="sparkler:allow_drawings"
                onchange={(event) => {
                    (globalThis as any).update_kv("sparkler:allow_drawings", (event.target as any).checked.toString());
                }}
            />

            <label for="sparkler:allow_drawings" class="normal">
                {lang["settings:account.html:label.allow_drawings"]}
            </label>
        </div>
    </div>

    <h4 class="title">{lang["settings:account.html:title.my_account"]}</h4>
    <form class="flex flex-col gap-1" id="change_username">
        <b class="heading">{lang["settings:account.html:label.change_username"]}</b>

        <label for="current_password_username">{lang["settings:account.html:label.current_password"]}</label>

        <input type="password" name="current_password_username" id="current_password_username" />

        <label for="new_name">New username</label>
        <input type="text" name="new_name" id="new_name" minlength="2" />

        <button>{lang["general:form.submit"]}</button>
    </form>

    <form class="flex flex-col gap-1" id="change_password">
        <b class="heading">{lang["settings:account.html:label.change_password"]}</b>

        <label for="current_password">{lang["settings:account.html:label.current_password"]}</label>
        <input type="password" name="current_password" id="current_password" />

        <label for="new_password">{lang["settings:account.html:label.new_password"]}</label>
        <input type="password" name="new_password" id="new_password" minlength="6" />

        <button>{lang["general:form.submit"]}</button>
    </form>

    <form class="flex flex-col gap-1" id="delete_account">
        <b class="heading">{lang["settings:account.html:label.delete_account"]}</b>

        <p class="fade subtext">
            {lang["settings:account.html:text.delete_account_warning"]}
        </p>

        <label for="current_password_delete">{lang["settings:account.html:label.current_password"]}</label>
        <input type="password" name="current_password_delete" id="current_password_delete" />

        <button>{lang["general:form.submit"]}</button>
    </form>

    <h4 class="title">{lang["settings:account.html:title.blocks"]}</h4>
    <div class="flex flex-col gap-1" id="sparkler:blocks">
        <b class="heading">{lang["settings:account.html:label.users"]}</b>
        <div class="card">
            <ul style="margin-bottom: 0">
                {#each relationships as relationship}
                    <li>
                        <div class="footernav" style="display: inline-flex">
                            <a href="/@{relationship[0].username}" class="item">
                                {relationship[0].username}
                            </a>

                            <span class="item">
                                <a href="javascript:remove_relationship('{relationship[0].username}')">
                                    {lang["settings:account.html:action.unblock"]}
                                </a>
                            </span>
                        </div>
                    </li>
                {/each}
            </ul>
        </div>

        <label for="sparkler:block_somebody">Block somebody</label>

        <div class="flex gap-2">
            <input name="sparkler:block_somebody" id="sparkler:block_somebody" placeholder="username" />

            <button
                type="button"
                onclick={() => {
                    (globalThis as any).block_dialog();
                }}
            >
                {lang["general:form.submit"]}
            </button>
        </div>

        <p class="fade">
            Put the username of somebody you want to block in the input above and click "Submit" to add them to your
            block list.
        </p>

        <b class="heading">{lang["settings:account.html:label.ips"]}</b>

        <p class="fade subtext">
            Some context is provided to help you remember why you created these blocks. The IP of each block will not be
            shown.
        </p>

        <div class="card">
            <ul style="margin-bottom: 0">
                {#each ipblocks as block}
                    <li>
                        <div class="footernav items-center" style="display: inline-flex">
                            <button
                                class="gap-2 round"
                                onclick={() => {
                                    (document.getElementById(`blockcontext:${block.id}`) as any).showModal();
                                }}
                            >
                                <Ellipsis class="icon" />
                                {lang["settings:account.html:text.context"]}
                            </button>

                            <dialog id="blockcontext:{block.id}">
                                <div class="inner" style="min-height: 250px">
                                    <div class="w-full flex justify-between items-center gap-2">
                                        <b>{lang["settings:account.html:text.context"]}</b>
                                        <div class="flex gap-2">
                                            <button
                                                class="bold red camo icon-only"
                                                onclick={() => {
                                                    (
                                                        document.getElementById(`blockcontext:${block.id}`) as any
                                                    ).close();
                                                }}
                                                type="button"
                                                title="Close"
                                            >
                                                <X class="icon" />
                                            </button>
                                        </div>
                                    </div>

                                    <hr class="flipped" />
                                    <span>{block.context}</span>
                                </div>
                            </dialog>

                            <span class="item fade"
                                >{lang["settings:account.html:text.blocked"]}
                                <span class="date">{block.timestamp}</span></span
                            >

                            <span class="item">
                                <a href="javascript:remove_ipblock('{block.id}')">
                                    {lang["settings:account.html:action.unblock"]}
                                </a>
                            </span>
                        </div>
                    </li>
                {/each}
            </ul>
        </div>
    </div>
</div>

<dialog id="block_dialog">
    <div class="inner">
        <p>{lang["settings:account.html:text.confirm_block"]}</p>

        <hr />
        <div class="flex gap-2">
            <button
                class="primary bold"
                onclick={() => {
                    (globalThis as any).block();
                }}
            >
                {lang["general:dialog.continue"]}
            </button>

            <button
                class="bold"
                onclick={() => {
                    (document.getElementById("block_dialog") as any).close();
                    window.close();
                }}
            >
                {lang["general:dialog.cancel"]}
            </button>
        </div>
    </div>
</dialog>

<dialog id="set_note_dialog">
    <script type="module" src="https://unpkg.com/emoji-picker-element@1.22.8/index.js"></script>

    <form class="inner">
        <textarea
            name="sparkler:status_note"
            id="sparkler:status_note"
            onchange={(event) => {
                (globalThis as any).update_kv("sparkler:status_note", (event.target as any).value);
            }}
            placeholder="Tell your friends what you're up to!"
        ></textarea>

        <p class="fade">{lang["settings:account.html:text.status_note"]}</p>

        <details id="emoji_details">
            <summary class="flex gap-2 items-center">
                <div id="sparkler:status_emoji"></div>
                {lang["settings:account.html:label.choose_emoji"]}
            </summary>

            <div class="flex gap-2">
                <div class="thread_line"></div>
                <emoji-picker
                    style="
                        --border-radius: var(--radius);
                        --background: var(--color-super-raised);
                        --input-border-radiFus: var(--radius);
                        --input-border-color: var(--color-primary);
                        --indicator-color: var(--color-primary);
                        --emoji-padding: 0.5rem;
                        box-shadow: 0 0 4px var(--color-shadow);
                    "
                    class="w-full"
                ></emoji-picker>
            </div>
        </details>
        <script>
            // I'm not making a whole emoji picker for this one thing, sorry
            document.querySelector("emoji-picker").addEventListener("emoji-click", (event) => {
                update_kv("sparkler:status_emoji", event.detail.unicode);
                document.getElementById("sparkler:status_emoji").innerText = event.detail.unicode;
                document.getElementById("emoji_details").removeAttribute("open");
            });

            setTimeout(() => {
                document.getElementById("sparkler:status_emoji").innerText =
                    document.getElementById("sparkler:status_emoji").value || "💭";
            }, 100);
        </script>

        <p class="fade">{lang["settings:account.html:text.status_emoji"]}</p>

        <hr />
        <div class="flex gap-2">
            <button
                class="primary bold"
                onclick={() => {
                    (globalThis as any).save_settings().then(() => window.close());
                }}
            >
                {lang["general:dialog.continue"]}
            </button>

            <button
                class="bold"
                onclick={() => {
                    (document.getElementById("set_note_dialog") as any).close();
                    window.close();
                }}
                type="button"
            >
                {lang["general:dialog.cancel"]}
            </button>
        </div>
    </form>
</dialog>

<dialog id="set_signature_dialog">
    <div class="inner">
        <textarea
            name="sparkler:mail_signature"
            id="sparkler:mail_signature"
            onchange={(event) => {
                (globalThis as any).update_kv("sparkler:mail_signature", (event.target as any).value);
            }}
        ></textarea>

        <p class="fade">{lang["settings:account.html:text.signature"]}</p>

        <hr />
        <div class="flex gap-2">
            <button
                class="primary bold"
                onclick={() => {
                    (globalThis as any).save_settings().then(() => window.close());
                }}
            >
                {lang["general:dialog.continue"]}
            </button>
            <button
                class="bold"
                onclick={() => {
                    (document.getElementById("set_signature_dialog") as any).close();
                    window.close();
                }}
                type="button"
            >
                {lang["general:dialog.cancel"]}
            </button>
        </div>
    </div>
</dialog>
