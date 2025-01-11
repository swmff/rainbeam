<script lang="ts">
    import { Option } from "$lib/classes/Option.js";

    import {
        Book,
        BookUser,
        ChevronDown,
        LogIn,
        LogOut,
        Mails,
        MessageCircleMore,
        Search,
        Settings,
        Store,
        UserRound,
        UserRoundPlus,
        UsersRound
    } from "lucide-svelte";
    import Dropdown from "$lib/components/Dropdown.svelte";
    import { onMount } from "svelte";

    const { children, data } = $props();
    const user = Option.from(data.user);
    const lang = data.lang;
    const config = data.config;

    onMount(async () => {
        const init = await import("$lib/init");
        init.default();
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

<div id="page">
    <div class="content_container" id="page_content">
        <article>
            <main class="flex flex-col gap-2">
                <nav class="items-center">
                    <div class="nav_side">
                        <a href="/" class="button title">
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
                    </div>

                    {#if user.is_some()}
                        {@const profile = user.unwrap()}
                        <div class="nav_side">
                            <Dropdown classname="title">
                                <div class="flex items-center gap-2">
                                    <img
                                        title="{profile.username}'s avatar"
                                        src="/api/v0/auth/profile/{profile.id}/avatar"
                                        alt=""
                                        class="avatar shadow"
                                        style="--size: 24px"
                                    />

                                    <ChevronDown class="icon dropdown-arrow" />
                                </div>

                                <div class="inner shadow-md">
                                    <b class="title">{profile.username}</b>

                                    <a href="/@{profile.username}">
                                        <UserRound class="icon" />
                                        {lang["general:link.show_profile"]}
                                    </a>

                                    <a href="/settings">
                                        <Settings class="icon" />
                                        {lang["general:link.settings"]}
                                    </a>

                                    <b class="title"
                                        >{lang["general:title.services"]}</b
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

                                    <b class="title"
                                        >{lang["general:title.social"]}</b
                                    >

                                    <a href="/@{profile.username}/friends">
                                        <BookUser class="icon" />
                                        {lang["general:link.friends"]}
                                    </a>

                                    <a
                                        href="/@{profile.username}/friends/requests"
                                    >
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
                                    <button
                                        onclick={trigger("app:logout")}
                                        class="red"
                                    >
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

                                <div class="inner shadow-md">
                                    <b class="title"
                                        >{lang["general:title.account"]}</b
                                    >

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
                </nav>

                <div style="padding: 0.5rem;" class="flex flex-col gap-2">
                    {@render children()}
                </div>
            </main>
        </article>
    </div>
</div>

<style>
    article,
    main {
        min-height: 100dvh;
    }

    main {
        border-left: solid 1px var(--color-super-lowered);
        border-right: solid 1px var(--color-super-lowered);
        padding: 0;
    }

    @media screen and (max-width: 900px) {
        article {
            padding: 0;
        }

        main {
            border: none;
        }
    }
</style>
