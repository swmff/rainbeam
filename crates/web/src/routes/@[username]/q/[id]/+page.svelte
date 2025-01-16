<script lang="ts">
    import { Option } from "$lib/classes/Option.js";
    import type { Profile } from "$lib/bindings/Profile.js";
    import { onMount } from "svelte";
    import Response from "$lib/components/Response.svelte";
    import { active_page } from "$lib/stores.js";
    import GlobalQuestion from "$lib/components/GlobalQuestion.svelte";

    const { data } = $props();
    active_page.set("response");

    const user = Option.from(data.user);
    const config = data.config;
    const lang = data.lang;
    const page = data.data;

    onMount(async () => {});

    const {
        question,
        responses,
        reactions,
        is_helper,
        is_powerful,
        already_responded
    } = page;
</script>

<svelte:head>
    <title>{config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

<article>
    <main class="flex flex-col gap-2">
        <GlobalQuestion
            ques={[question, responses.length, reactions.length]}
            profile={user}
            show_responses={false}
        />

        {#if is_powerful}
            <div class="question_ip card shadow round">
                <a href="/+i/{question.ip}">{question.ip}</a>
            </div>
        {/if}

        <hr />

        <div class="pillmenu convertible true">
            <a href="#/responses" class="active" data-tab-button="responses"
                ><span>{lang["views:text.responses"]}</span></a
            >
            <a href="#/reactions" data-tab-button="reactions"
                ><span>{lang["views:text.reactions"]}</span></a
            >
        </div>

        <div data-tab="responses" class="flex flex-col gap-4">
            {#if already_responded}
                <p class="fade">You've already responded to this question!</p>
            {:else}
                <div class="card-nest w-full">
                    <div class="card flex flex-col gap-1">Add a response</div>

                    <form
                        class="card flex flex-col gap-2"
                        onsubmit={(e) => {
                            e.preventDefault();
                            fetch("/api/v1/responses", {
                                method: "POST",
                                headers: {
                                    "Content-Type": "application/json"
                                },
                                body: JSON.stringify({
                                    question: question.id,
                                    content: (e.target as any).content.value
                                })
                            })
                                .then((res) => res.json())
                                .then((res) => {
                                    trigger("app:shout", [
                                        res.success ? "tip" : "caution",
                                        res.message || "Response posted!"
                                    ]);

                                    document
                                        .getElementById(`question:${question}`)!
                                        .setAttribute("disabled", "fully");

                                    if (res.success === true) {
                                        (e.target as any).reset();
                                        document
                                            .getElementById(
                                                `question:${question}`
                                            )!
                                            .removeAttribute("disabled");
                                    }
                                });
                        }}
                    >
                        <textarea
                            class="w-full"
                            placeholder="Type your response!"
                            minlength="1"
                            maxlength="4096"
                            required
                            name="content"
                            id="content"
                            data-hook="counter"
                        ></textarea>

                        <div class="flex justify-between w-full gap-1">
                            <span id="content:counter" class="notification item"
                            ></span>
                            <button class="primary bold">
                                {lang["general:form.submit"]}
                            </button>
                        </div>
                    </form>
                </div>
            {/if}

            {#each responses as res}
                <Response
                    {res}
                    {is_helper}
                    {is_powerful}
                    profile={user}
                    {lang}
                    {config}
                    show_comments={true}
                    is_pinned={false}
                    show_pin_button={false}
                    do_render_nested={true}
                    anonymous_avatar={""}
                    anonymous_username={""}
                />
            {/each}

            <div class="flex justify-between gap-2 w-full">
                {#if page.page > 0}
                    <a
                        class="button secondary"
                        href="?page={page.page - 1}"
                        data-sveltekit-reload
                    >
                        {lang["general:link.previous"]}
                    </a>
                {:else}
                    <div></div>
                {/if}

                {#if responses.length != 0}
                    <a
                        class="button secondary"
                        href="?page={page.page + 1}"
                        data-sveltekit-reload
                    >
                        {lang["general:link.next"]}
                    </a>
                {/if}
            </div>
        </div>

        <div data-tab="reactions" class="hidden">
            <div id="reactions" class="card shadow flex gap-2 flex-col w-full">
                {#each reactions as reaction}
                    <a
                        href="/@{reaction.user.username}"
                        class="card w-full flex items-center gap-2"
                    >
                        <img
                            title="{reaction.user.username}'s avatar"
                            src="/api/v0/auth/profile/{reaction.user.id}/avatar"
                            alt="@{reaction.user.username}"
                            class="avatar"
                            loading="lazy"
                            style="--size: 20px"
                        />
                        {reaction.user.username}
                    </a>
                {/each}
            </div>
        </div>
    </main>
</article>
