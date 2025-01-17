<script lang="ts">
    import { Option } from "$lib/classes/Option";

    const { children, data } = $props();
    const user = Option.from(data.user).unwrap();
    const lang = data.lang;
    const page = data.data;
    const config = data.config;

    const { viewing_other_profile } = page;

    import { active_page } from "$lib/stores.js";
    import { onMount } from "svelte";

    let active = $state("");

    onMount(() => {
        active_page.subscribe((v) => {
            active = v;
        });

        setTimeout(() => {
            const app = ns("app");
            let metadata = user.metadata;

            // handle update
            (globalThis as any).update_kv = (key: string, value: string) => {
                metadata.kv[key] = value;
                (globalThis as any).save_settings(); // auto save

                // live theme
                if (key.startsWith("sparkler:color_")) {
                    const real_key = key.replace("sparkler:", "");
                    const css_var = real_key.replaceAll("_", "-");
                    value = value
                        .replaceAll(";", "")
                        .replaceAll("}", "")
                        .replaceAll("<", "%lt;")
                        .replaceAll(">", "%gt;");

                    // check for existing stylesheet
                    const existing = document.getElementById(`sparkler_live:${real_key}`);

                    if (existing) {
                        if (value === "") {
                            // use default
                            existing.remove();
                            return;
                        }

                        existing.innerHTML = `:root, * { --${css_var}: ${value} !important }`;
                    } else {
                        const stylesheets = document.getElementById("stylesheets");
                        const stylesheet = document.createElement("style");
                        stylesheet.id = `sparkler_live:${real_key}`;
                        stylesheet.innerHTML = `:root, * { --${css_var}: ${value} !important }`;
                        (stylesheets as any).appendChild(stylesheet);
                    }
                }
            };

            // handle colors
            (globalThis as any).link_color = (id: string, color: string) => {
                (document.getElementById(id) as HTMLInputElement).value = color;
                (globalThis as any).update_kv(id, color);
            };

            // prefill
            setTimeout(() => {
                for (const [key, value] of Object.entries(metadata.kv)) {
                    if (key.length === 0) {
                        continue;
                    }

                    if (document.getElementById(key)) {
                        (document.getElementById(key) as HTMLInputElement).value = value as string;

                        if (value === "true") {
                            (document.getElementById(key) as HTMLElement).setAttribute("checked", "true");
                        }
                    }
                }

                for (let [key, value] of Object.entries(window.localStorage)) {
                    if (key.length === 0) {
                        continue;
                    }

                    key = `sparkler:${key}`;
                    if (document.getElementById(key)) {
                        (document.getElementById(key) as HTMLInputElement).value = value;

                        if (value === "true") {
                            (document.getElementById(key) as HTMLElement).setAttribute("checked", "true");
                        }
                    }
                }
            }, 50);

            // handle submit
            (globalThis as any).save_settings = async () => {
                await app.debounce("settings:save_settings");
                const res = await (
                    await fetch(`/api/v0/auth/profile/${user.id}/metadata`, {
                        method: "POST",
                        headers: {
                            "Content-Type": "application/json"
                        },
                        body: JSON.stringify({
                            metadata
                        })
                    })
                ).json();

                trigger("app:toast", [
                    res.success ? "success" : "error",
                    res.success ? "Settings saved!" : res.message
                ]);
            };
        }, 100);
    });
</script>

<svelte:head>
    <title>Settings - {config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

<article class="flex flex-collapse gap-2">
    <div
        id="settings_nav"
        class="flex flex-col gap-4 sm:static sm:w-full"
        style="
            width: 20rem;
            padding-top: 0;
            height: max-content;
            top: calc(64px + 0.5rem);
            position: sticky;
        "
    >
        <div class="sidenav shadow">
            <a class={active === "settings.account" ? "active" : ""} href="/settings?profile={user.id}"
                >{lang["settings:link.account"]}</a
            >
            <a class={active === "settings.sessions" ? "active" : ""} href="/settings/sessions?profile={user.id}"
                >{lang["settings:link.sessions"]}</a
            >
            <a class={active === "settings.profile" ? "active" : ""} href="/settings/profile?profile={user.id}"
                >{lang["settings:link.profile"]}</a
            >
            <a class={active === "settings.privacy" ? "active" : ""} href="/settings/privacy?profile={user.id}"
                >{lang["settings:link.privacy"]}</a
            >
            <a class={active === "settings.coins" ? "active" : ""} href="/settings/coins?profile={user.id}"
                >{lang["settings:link.coins"]}</a
            >
        </div>
    </div>

    <section id="settings_content" class="card shadow flex flex-col gap-2 w-full">
        {#if viewing_other_profile}
            <div class="markdown-alert-warning">
                <span>
                    These are not your user settings.
                    <b>Attempting to delete the account through this page will delete your account</b>!
                </span>
            </div>
        {/if}

        <div id="admonition_zone"></div>
        {@render children()}
    </section>
</article>

<!-- svelte-ignore css_unused_selector -->
<style>
    label:not(.checkbox_container *):not(dialog *),
    .setting {
        text-transform: uppercase;
        opacity: 75%;
        font-weight: 700;
        font-size: 14px;
    }

    .heading {
        text-transform: title;
        font-weight: 700;
        opacity: 100%;
    }

    label + * + p.fade,
    p.fade.subtext {
        font-size: 13px;
    }

    .title:not(:first-of-type):not(.inner *) {
        margin-top: 2rem;
    }
</style>
