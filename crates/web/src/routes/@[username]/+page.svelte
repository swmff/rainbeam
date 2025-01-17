<script lang="ts">
    import { onMount } from "svelte";

    import { active_page } from "$lib/stores.js";
    active_page.set("profile");

    import { Option } from "$lib/classes/Option";
    import Response from "$lib/components/Response.svelte";
    import Scroller from "$lib/components/Scroller.svelte";

    const { data } = $props();
    const lang = data.lang;
    const page_data = data.data;
    const user = Option.from(data.user);
    const config = data.config;
    const search_query = data.query;

    const { other, page, is_helper, pinned, response_count, questions_count } = page_data;

    async function load_responses() {
        return await (await fetch(`/_partial/profile/${other.username}?page=${search_query.page || "0"}`)).json();
    }

    let responses = $state([] as any[]);
    onMount(async () => {
        for (const res of (await load_responses()).payload.responses) {
            responses.push(res);
        }

        // partial
        setTimeout(() => {
            trigger("questions:carp");
        }, 100);
    });
</script>

<div class="pillmenu convertible shadow">
    <a href="/@{other.username}" class="active">
        <span
            >{lang["profile:link.feed"]}
            <b class="notification">{response_count}</b></span
        >
    </a>

    <a href="/@{other.username}/questions">
        <span>Questions <b class="notification">{questions_count}</b></span>
    </a>

    {#if is_helper}
        <a href="/@{other.username}/mod">
            <span>{lang["profile:link.manage"]} <b class="notification">Mod</b></span>
        </a>
    {/if}
</div>

<div id="feed" class="flex flex-col gap-2">
    {#if pinned}
        {#each pinned as res}
            <Response
                {res}
                anonymous_avatar={other.metadata.kv["sparkler:anonymous_avatar"] || ""}
                anonymous_username={other.metadata.kv["sparkler:anonymous_username"] || ""}
                is_powerful={page.is_powerful}
                is_helper={page.is_helper}
                is_pinned={true}
                show_pin_button={true}
                do_render_nested={true}
                show_comments={true}
                profile={user}
                {lang}
                {config}
            />
        {/each}
    {/if}

    {#each responses as res}
        <Response
            {res}
            anonymous_avatar={other.metadata.kv["sparkler:anonymous_avatar"] || ""}
            anonymous_username={other.metadata.kv["sparkler:anonymous_username"] || ""}
            is_powerful={page.is_powerful}
            is_helper={page.is_helper}
            is_pinned={false}
            show_pin_button={true}
            do_render_nested={true}
            show_comments={true}
            profile={user}
            {lang}
            {config}
        />
    {/each}

    <Scroller
        threshold={100}
        load={async () => {
            if (search_query.page) {
                search_query.page += 1;
            } else {
                search_query.page = 1;
            }

            for (const res of (await load_responses()).payload.responses) {
                responses.push(res);
            }

            setTimeout(() => {
                (globalThis as any).__init();
            }, 100);
        }}
    />
</div>
