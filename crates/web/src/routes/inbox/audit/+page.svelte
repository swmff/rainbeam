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
    const config = data.config;

    const { logs, page } = page_data;
</script>

<svelte:head>
    <title>Audit log - {config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

<article>
    <main class="flex flex-col gap-2">
        <div class="pillmenu convertible shadow">
            <a href="/inbox"><span>My Inbox</span></a>
            <a href="/inbox/audit" class="active"><span>Audit Log</span></a>
            <a href="/inbox/reports"><span>Reports</span></a>
        </div>

        {#if logs.length === 0}
            <div class="markdown-alert-warning">
                <span>{lang["general:text.no_results"]}</span>
            </div>
        {:else}
            {#each logs as notif}
                <Notification {notif} {lang} show_mark_as_read={false} />
            {/each}

            <div class="flex justify-between gap-2 w-full">
                {#if page > 0}
                    <a class="button secondary" href="?page={data.query.page - 1}" data-sveltekit-reload> Previous </a>
                {:else}
                    <div></div>
                {/if}

                {#if logs.length !== 0}
                    <a class="button secondary" href="?page={data.query.page + 1}" data-sveltekit-reload> Next </a>
                {/if}
            </div>
        {/if}
    </main>
</article>
