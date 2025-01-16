<script lang="ts">
    import { Bomb, Ellipsis, Flag, Shield, Trash } from "lucide-svelte";
    import { onMount } from "svelte";

    import { active_page } from "$lib/stores.js";
    active_page.set("inbox");

    import Question from "$lib/components/Question.svelte";
    import Dropdown from "$lib/components/Dropdown.svelte";
    import MoreResponseOptions from "$lib/components/MoreResponseOptions.svelte";
    import { Option } from "$lib/classes/Option";
    import Notification from "$lib/components/Notification.svelte";

    const { data } = $props();
    const lang = data.lang;
    const page_data = data.data;
    const user = Option.from(data.user);
    const profile = user.unwrap();
    const config = data.config;

    const { notifs, pid, page } = page_data;
</script>

<svelte:head>
    <title>Notifications - {config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

<article>
    <main class="flex flex-col gap-2">
        {#if profile.id !== pid}
            <b>{pid}</b>
        {/if}

        {#if notifs.length === 0}
            <div class="markdown-alert-warning">
                <span>{lang["general:text.no_results"]}</span>
            </div>
        {:else}
            <div class="w-full flex justify-between">
                <div></div>
                <button
                    class="red"
                    onclick={() => {
                        trigger("notifications:clear", []);
                    }}
                >
                    <Bomb class="icon" />
                    {lang["general:action.clear"]}
                </button>
            </div>

            {#each notifs as notif}
                <Notification {notif} {lang} show_mark_as_read={true} />
            {/each}

            <div class="flex justify-between gap-2 w-full">
                {#if page > 0}
                    <a
                        class="button secondary"
                        href="?page={page - 1}&profile={pid}"
                        data-sveltekit-reload
                    >
                        Previous
                    </a>
                {:else}
                    <div></div>
                {/if}

                {#if notifs.length !== 0}
                    <a
                        class="button secondary"
                        href="?page={page + 1}&profile={pid}"
                        data-sveltekit-reload
                    >
                        Next
                    </a>
                {/if}
            </div>
        {/if}
    </main>
</article>
