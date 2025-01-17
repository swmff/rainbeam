<script lang="ts">
    import { active_page } from "$lib/stores.js";
    active_page.set("profile");

    import Question from "$lib/components/Question.svelte";

    const { data } = $props();
    const lang = data.lang;
    const page_data = data.data;

    const { other, is_helper, is_self, is_powerful, response_count, questions_count, questions } = page_data;
</script>

<div class="pillmenu convertible shadow">
    <a href="/@{other.username}">
        <span
            >{lang["profile:link.feed"]}
            <b class="notification">{response_count}</b></span
        >
    </a>

    <a href="/@{other.username}/questions" class="active">
        <span>Questions <b class="notification">{questions_count}</b></span>
    </a>

    {#if is_helper}
        <a href="/@{other.username}/mod">
            <span>{lang["profile:link.manage"]} <b class="notification">Mod</b></span>
        </a>
    {/if}
</div>

{#if is_self || is_powerful}
    <div class="pillmenu convertible shadow">
        <a href="/@{other.username}/questions"><span>{lang["timelines:link.global"]}</span></a>

        {#if is_powerful}
            <a href="/@{other.username}/questions/inbox">
                <span
                    >{lang["general:link.inbox"]}
                    <b class="notification">Mod</b></span
                >
            </a>
        {/if}

        <a href="/@{other.username}/questions/outbox" class="active">
            <span
                >{lang["profile:link.outbox"]}
                <b class="notification">{lang["profile:label.private"]}</b></span
            >
        </a>
    </div>
{/if}

<div id="feed" class="flex flex-col gap-2">
    {#each questions as question}
        <Question {question} actions={null} />
    {/each}
</div>
