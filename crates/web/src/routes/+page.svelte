<script lang="ts">
    import { Option, Some } from "$lib/classes/Option.js";
    import type { Profile } from "$lib/bindings/Profile.js";
    import { onMount } from "svelte";
    import BigFriend from "$lib/components/BigFriend.svelte";
    import Response from "$lib/components/Response.svelte";
    import Scroller from "$lib/components/Scroller.svelte";
    import { active_page } from "$lib/stores.js";
    import { Smile, UserRoundPlus } from "lucide-svelte";
    import LangPicker from "$lib/components/LangPicker.svelte";

    const { data } = $props();
    active_page.set("timeline");

    const user = Option.from(data.user);
    const config = data.config;
    const lang = data.lang;
    const page = data.data;
    const query = data.query;

    async function load_responses() {
        return await (
            await fetch(`/_partial/timeline?page=${query.page || "0"}`)
        ).json();
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

<svelte:head>
    <title>{config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

{#if user.is_some()}
    <article>
        <main class="flex flex-col gap-2">
            <div class="pillmenu convertible">
                <a href="/" class="active"
                    ><span>{lang["timelines:link.timeline"]}</span></a
                >
                <a href="/inbox/posts"
                    ><span>{lang["timelines:link.posts"]}</span></a
                >
                <a href="/inbox/global"
                    ><span>{lang["timelines:link.global"]}</span></a
                >
            </div>

            <div class="pillmenu convertible">
                <a href="/public"
                    ><span>{lang["timelines:link.public"]}</span></a
                >
                <a href="/" class="active"
                    ><span>{lang["timelines:link.following"]}</span></a
                >
            </div>

            {#if user.is_some()}
                {@const profile = user.unwrap() as Profile}
                <div class="card w-full flex flex-col gap-2">
                    <h5 id="friends">My Friends</h5>
                    <div class="flex gap-2 flex-wrap">
                        <BigFriend
                            user={profile}
                            profile={Some(profile)}
                            {lang}
                        />
                        {#each page.friends as user}
                            {#if profile.id !== user[0].id}
                                <BigFriend
                                    user={user[0]}
                                    profile={Some(profile)}
                                    {lang}
                                />
                            {:else}
                                <BigFriend
                                    user={user[1]}
                                    profile={Some(profile)}
                                    {lang}
                                />
                            {/if}
                        {/each}
                    </div>
                </div>

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
                            do_render_nested={true}
                            show_comments={true}
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
{:else}
    <div class="w-full flex flex-col items-center" style="margin-top: 2rem">
        <div class="flex flex-col items-center gap-2">
            <h1 class="no-margin" style="color: var(--color-primary)">
                {config.name}
            </h1>

            <h3 style="font-weight: normal; margin-top: 0">
                {config.description}
            </h3>
        </div>

        <div
            class="flex flex-col gap-4 items-center justify-center"
            style="width: 20rem; max-width: 100%"
        >
            <hr class="w-full" />

            <div class="flex flex-col gap-2 w-full">
                <a
                    class="big primary button bold w-full"
                    href="/sign_up"
                    data-turbo="false"
                    style="gap: 1rem !important"
                >
                    <UserRoundPlus class="icon" />
                    {lang["homepage.html:link.create_account"]}
                </a>

                <a
                    class="big button secondary bold w-full"
                    href="/login"
                    data-turbo="false"
                    style="gap: 1rem !important"
                >
                    <Smile class="icon" />{lang["general:link.login"]}
                </a>

                <LangPicker lang={data.lang_name} />
            </div>
        </div>
    </div>
{/if}
