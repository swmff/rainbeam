<script lang="ts">
    import { Option } from "$lib/classes/Option.js";
    import type { Profile } from "$lib/bindings/Profile.js";
    import { onMount } from "svelte";
    import Response from "$lib/components/Response.svelte";
    import Scroller from "$lib/components/Scroller.svelte";
    import { active_page } from "$lib/stores.js";

    const { data } = $props();
    active_page.set("timeline");

    const user = Option.from(data.user);
    const config = data.config;
    const lang = data.lang;
    const page = data.data;
    const query = data.query;

    async function load_responses() {
        return await (
            await fetch(`/_partial/posts?page=${query.page || "0"}`)
        ).json();
    }

    let responses = $state([] as any[]);
    onMount(async () => {
        for (const res of (await load_responses()).payload.responses) {
            responses.push(res);
        }

        setTimeout(() => {
            (globalThis as any).__init();
        }, 100);

        // partial
        setTimeout(() => {
            trigger("questions:carp");
        }, 100);
    });
</script>

<svelte:head>
    <title>{config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

<article>
    <main class="flex flex-col gap-2">
        <div class="pillmenu convertible">
            <a href="/"><span>{lang["timelines:link.timeline"]}</span></a>
            <a href="/inbox/posts" class="active"
                ><span>{lang["timelines:link.posts"]}</span></a
            >
            <a href="/inbox/global"
                ><span>{lang["timelines:link.global"]}</span></a
            >
        </div>

        <div class="pillmenu convertible">
            <a href="/inbox/posts" class="active"
                ><span>{lang["timelines:link.public"]}</span></a
            >
            <a href="/inbox/posts/following"
                ><span>{lang["timelines:link.following"]}</span></a
            >
        </div>

        {#if user.is_some()}
            {@const profile = user.unwrap() as Profile}
            <div id="feed" class="flex flex-col gap-2">
                {#each responses as res}
                    <Response
                        {res}
                        anonymous_avatar={profile.metadata.kv[
                            "sparkler:anonymous_avatar"
                        ] || ""}
                        anonymous_username={profile.metadata.kv[
                            "sparkler:anonymous_username"
                        ] || ""}
                        is_powerful={page.is_powerful}
                        is_helper={page.is_helper}
                        is_pinned={false}
                        show_pin_button={false}
                        show_comments={true}
                        do_render_nested={true}
                        profile={user}
                        {lang}
                        {config}
                    />
                {/each}
            </div>

            <Scroller
                threshold={100}
                load={async () => {
                    if (query.page) {
                        query.page += 1;
                    } else {
                        query.page = 1;
                    }

                    for (const res of (await load_responses()).payload
                        .responses) {
                        responses.push(res);
                    }
                }}
            />
        {/if}
    </main>
</article>
