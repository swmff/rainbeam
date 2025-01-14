<script lang="ts">
    import type { LayoutData } from "./+layout.server";
    const { children, data }: { children: any; data: LayoutData } = $props();

    import { Option } from "$lib/classes/Option.js";
    import type { Profile } from "$lib/bindings/Profile.js";
    import { active_page } from "$lib/stores.js";

    import {
        Bell,
        Book,
        BookUser,
        Check,
        ChevronDown,
        House,
        Inbox,
        LogIn,
        LogOut,
        Mails,
        MessageCircleMore,
        PenSquare,
        Search,
        Settings,
        Store,
        UserRound,
        UserRoundPlus,
        UsersRound,
        X
    } from "lucide-svelte";

    import Dropdown from "$lib/components/Dropdown.svelte";
    import { onMount } from "svelte";

    const user = Option.from(data.user) as Option<Profile>;
    const lang = data.lang;
    const config = data.config;

    const notifs = data.notifs;
    const unread = data.unread;

    let active = $state("");

    onMount(async () => {
        if ((globalThis as any).__scroll_event) {
            (globalThis as any).__scroll_event = undefined;
            document.body.removeEventListener(
                "scroll",
                (globalThis as any).__scroll_event
            );
        }

        const init = await import("$lib/init");
        init.default();

        (globalThis as any).__init = init.default;
        (globalThis as any).__user = user;

        active_page.subscribe((v) => {
            active = v;
        });
    });
</script>

<svelte:head>
    <link rel="stylesheet" href="/css/style.css" />

    <script lang="js">
        globalThis.ns_verbose = false;
        globalThis.ns_config = {
            root: "",
            verbose: globalThis.ns_verbose
        };

        globalThis._app_base = {
            name: "dust",
            api: "rainbeam",
            ns_store: {},
            classes: {}
        };

        globalThis.no_policy = false;
    </script>

    <script lang="ts">
        function media_theme_pref() {
            document.documentElement.removeAttribute("class");

            if (
                window.matchMedia("(prefers-color-scheme: dark)").matches &&
                !window.localStorage.getItem("theme")
            ) {
                document.documentElement.classList.add("dark");
            } else if (
                window.matchMedia("(prefers-color-scheme: light)").matches &&
                !window.localStorage.getItem("theme")
            ) {
                document.documentElement.classList.remove("dark");
            } else if (window.localStorage.getItem("theme")) {
                const current = window.localStorage.getItem("theme");
                document.documentElement.className = current;
            }
        }

        media_theme_pref();
        document.documentElement.addEventListener("load", () => {
            if (!document.getElementById("theme")) {
                return;
            }

            const profile_theme = document
                .getElementById("theme")
                .innerText.trim();

            if (profile_theme) {
                return;
            }

            media_theme_pref();
        });
    </script>
</svelte:head>

{#if !data.layout_skip}
    <div id="toast_zone"></div>

    <nav class="items-center">
        <div class="content_container">
            <div class="nav_side flex gap-1">
                <a href="/" class="button title desktop">
                    <img
                        src="/images/ui/logo.svg"
                        alt={config.name}
                        width="32px"
                        height="32px"
                        class="title-content"
                        id="title-img"
                    />

                    <b class="title-content" style="display: none"
                        >{config.name}</b
                    >
                </a>

                <a
                    class="button {active === 'timeline' ? 'active' : ''}"
                    href="/"
                    title="Timeline"
                >
                    <House class="icon" />
                    <span class="desktop">{lang["general:link.timeline"]}</span>
                    <span class="mobile">{lang["general:link.home"]}</span>
                </a>

                <a
                    class="button {active === 'inbox' ? 'active' : ''}"
                    href="/inbox"
                    title="My inbox"
                >
                    <Inbox class="icon" />
                    <span class="flex items-center gap-2">
                        <span>{lang["general:link.inbox"]}</span>
                        {#if unread}
                            <span class="notification tr camo">{unread}</span>
                        {/if}
                    </span>
                </a>
            </div>

            {#if user.is_some()}
                {@const profile = user.unwrap()}
                <div class="nav_side flex gap-1">
                    <a
                        class="button {active === 'notifications'
                            ? 'active'
                            : ''}"
                        href="/inbox/notifications"
                        title="My notifications"
                    >
                        <Bell class="icon" />
                        {#if notifs}
                            <span class="notification tr camo">{notifs}</span>
                        {/if}
                    </a>

                    <a
                        class="button {active === 'compose' ? 'active' : ''}"
                        href="/intents/post"
                        title="Create post"
                    >
                        <PenSquare class="icon" />
                    </a>

                    <Dropdown classname="title">
                        <button class="camo flex items-center gap-2">
                            <img
                                title="{profile.username}'s avatar"
                                src="/api/v0/auth/profile/{profile.id}/avatar"
                                alt=""
                                class="avatar"
                                style="--size: 24px"
                            />

                            <ChevronDown class="icon dropdown-arrow" />
                        </button>

                        <div class="inner">
                            <b class="title">{profile.username}</b>

                            <a href="/@{profile.username}">
                                <UserRound class="icon" />
                                {lang["general:link.show_profile"]}
                            </a>

                            <a href="/settings">
                                <Settings class="icon" />
                                {lang["general:link.settings"]}
                            </a>

                            <b class="title">{lang["general:title.services"]}</b
                            >

                            <a href="/market?status=Featured">
                                <Store class="icon" />
                                {lang["general:service.market"]}
                            </a>

                            <a href="/chats">
                                <MessageCircleMore class="icon" />
                                {lang["general:service.chats"]}
                            </a>

                            <a href="/inbox/mail">
                                <Mails class="icon" />
                                {lang["general:service.mail"]}
                            </a>

                            <a href="/circles">
                                <UsersRound class="icon" />
                                {lang["general:service.circles"]}
                            </a>

                            <b class="title">{lang["general:title.social"]}</b>

                            <a href="/@{profile.username}/friends">
                                <BookUser class="icon" />
                                {lang["general:link.friends"]}
                            </a>

                            <a href="/@{profile.username}/friends/requests">
                                <UserRoundPlus class="icon" />
                                {lang["general:link.requests"]}
                            </a>

                            <b class="title">{config.name}</b>
                            <a href="https://swmff.github.io/rainbeam/">
                                <Book class="icon" />
                                {lang["base.html:link.reference"]}
                            </a>

                            <a href="/search">
                                <Search class="icon" />
                                {lang["general:link.search"]}
                            </a>

                            <b class="title"></b>
                            <button onclick={trigger("app:logout")} class="red">
                                <LogOut class="icon" />
                                {lang["base.html:link.sign_out"]}
                            </button>
                        </div>
                    </Dropdown>
                </div>
            {:else}
                <div class="nav_side">
                    <Dropdown classname="title">
                        <div class="flex items-center gap-2">
                            <ChevronDown class="icon dropdown-arrow" />
                        </div>

                        <div class="inner">
                            <b class="title">{lang["general:title.account"]}</b>

                            <a href="/login">
                                <LogIn class="icon" />
                                {lang["general:link.login"]}
                            </a>

                            <a href="/sign_up">
                                <UserRoundPlus class="icon" />
                                {lang["general:link.sign_up"]}
                            </a>
                        </div>
                    </Dropdown>
                </div>
            {/if}
        </div>
    </nav>

    <div id="page">
        <div class="content_container" id="page_content">
            <div style="padding: 0.5rem;" class="flex flex-col gap-2">
                {@render children()}
            </div>
        </div>
    </div>

    <!-- dialogs -->
    <dialog id="link_filter">
        <div class="inner">
            <p>Pressing continue will bring you to the following URL:</p>
            <pre><code id="link_filter_url"></code></pre>
            <p>Are sure you want to go there?</p>

            <hr />
            <div class="flex gap-2">
                <a
                    class="button primary bold"
                    id="link_filter_continue"
                    rel="noopener noreferrer"
                    target="_blank"
                    onclick={() => {
                        (
                            document.getElementById(
                                "link_filter"
                            ) as HTMLDialogElement
                        ).close();
                    }}
                    onkeydown={() => {}}
                    role="button"
                    tabindex="0"
                >
                    {lang["general:dialog.continue"]}
                </a>
                <button
                    class="bold"
                    type="button"
                    onclick={() => {
                        (
                            document.getElementById(
                                "link_filter"
                            ) as HTMLDialogElement
                        ).close();
                    }}
                >
                    {lang["general:dialog.cancel"]}
                </button>
            </div>
        </div>
    </dialog>

    <dialog id="web_api_prompt">
        <div class="inner flex flex-col gap-2">
            <form
                class="flex gap-2 flex-col"
                onsubmit={(event) => {
                    event.preventDefault();
                }}
            >
                <label for="prompt" id="web_api_prompt:msg"></label>
                <input id="prompt" name="prompt" />

                <div class="flex justify-between">
                    <div></div>

                    <div class="flex gap-2">
                        <button
                            class="primary bold circle"
                            onclick={() => {
                                (globalThis as any).web_api_prompt_submit(
                                    (
                                        document.getElementById(
                                            "prompt"
                                        ) as HTMLInputElement
                                    ).value
                                );

                                (
                                    document.getElementById(
                                        "prompt"
                                    ) as HTMLInputElement
                                ).value = "";
                            }}
                            type="button"
                        >
                            <Check class="icon" />
                            {lang["general:dialog.okay"]}
                        </button>

                        <button
                            class="bold red camo"
                            onclick={() => {
                                (globalThis as any).web_api_prompt_submit("");
                            }}
                            type="button"
                        >
                            <X class="icon" />
                            {lang["general:dialog.cancel"]}
                        </button>
                    </div>
                </div>
            </form>
        </div>
    </dialog>

    <dialog id="web_api_prompt_long">
        <div class="inner flex flex-col gap-2">
            <form
                class="flex gap-2 flex-col"
                onsubmit={(event) => {
                    event.preventDefault();
                }}
            >
                <label for="prompt_long" id="web_api_prompt_long:msg"></label>
                <textarea id="prompt_long" name="prompt_long"></textarea>

                <div class="flex justify-between">
                    <div></div>

                    <div class="flex gap-2">
                        <button
                            class="primary bold circle"
                            onclick={() => {
                                (globalThis as any).web_api_prompt_long_submit(
                                    (
                                        document.getElementById(
                                            "prompt_long"
                                        ) as HTMLTextAreaElement
                                    ).value
                                );

                                (
                                    document.getElementById(
                                        "prompt_long"
                                    ) as HTMLTextAreaElement
                                ).value = "";
                            }}
                            type="button"
                        >
                            <Check class="icon" />
                            {lang["general:dialog.okay"]}
                        </button>

                        <button
                            class="bold red camo"
                            onclick={() => {
                                (globalThis as any).web_api_prompt_long_submit(
                                    ""
                                );
                            }}
                            type="button"
                        >
                            <X class="icon" />
                            {lang["general:dialog.cancel"]}
                        </button>
                    </div>
                </div>
            </form>
        </div>
    </dialog>

    <dialog id="web_api_confirm">
        <div class="inner flex flex-col gap-2">
            <form
                class="flex gap-2 flex-col"
                onsubmit={(event) => {
                    event.preventDefault();
                }}
            >
                <b id="web_api_confirm:msg"></b>

                <div class="flex justify-between">
                    <div></div>

                    <div class="flex gap-2">
                        <button
                            class="primary bold circle"
                            onclick={() => {
                                (globalThis as any).web_api_confirm_submit(
                                    true
                                );
                            }}
                            type="button"
                        >
                            <Check class="icon" />
                            {lang["general:dialog.yes"]}
                        </button>

                        <button
                            class="bold red camo"
                            onclick={() => {
                                (globalThis as any).web_api_confirm_submit(
                                    false
                                );
                            }}
                            type="button"
                        >
                            <X class="icon" />
                            {lang["general:dialog.no"]}
                        </button>
                    </div>
                </div>
            </form>
        </div>
    </dialog>
{:else}
    {@render children()}
{/if}
