<script lang="ts">
    import { Option, Some } from "$lib/classes/Option.js";
    import type { Profile } from "$lib/bindings/Profile.js";
    import { onMount } from "svelte";
    import BigFriend from "$lib/components/BigFriend.svelte";
    import Response from "$lib/components/Response.svelte";

    const { data } = $props();

    const user = Option.from(data.user);
    const config = data.data;
    const lang = data.lang;
    const page = data.data;

    async function load_responses() {
        return await (await fetch("/_partial/timeline")).json();
    }

    let responses = $state([] as any[]);
    onMount(async () => {
        for (const res of (await load_responses()).payload.responses) {
            responses.push(res);
        }

        setTimeout(() => {
            (globalThis as any).__init();
        }, 100);
    });
</script>

{#if user.is_some()}
    {@const profile = user.unwrap() as Profile}
    <div class="card w-full flex flex-col gap-2">
        <h5 id="friends">My Friends</h5>
        <div class="flex gap-2 flex-wrap">
            <BigFriend user={profile} profile={Some(profile)} {lang} />
            {#each page.friends as user}
                {#if profile.id !== user[0].id}
                    <BigFriend user={user[0]} profile={Some(profile)} {lang} />
                {:else}
                    <BigFriend user={user[1]} profile={Some(profile)} {lang} />
                {/if}
            {/each}
        </div>
    </div>

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
            do_render_nested={true}
            profile={user}
            {lang}
            {config}
        />
    {/each}
{/if}
