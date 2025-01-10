<script lang="ts">
    import type { Profile } from "$lib/bindings/Profile.js";
    import { None, Option } from "$lib/classes/Option.js";

    const { children, data } = $props();
    const user = Option.from(data.user);
</script>

<svelte:head>
    <link rel="stylesheet" href="/css/style.css" />

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
                <nav>
                    <div class="nav_side">rainbeam</div>

                    {#if user.is_some()}
                        {@const profile = user.unwrap()}
                        <div class="nav_side">
                            <img
                                title="{profile.username}'s avatar"
                                src="/api/v0/auth/profile/{profile.id}/avatar"
                                alt=""
                                class="avatar shadow"
                                style="--size: 24px"
                            />
                        </div>
                    {:else}
                        <div class="nav_side"></div>
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
