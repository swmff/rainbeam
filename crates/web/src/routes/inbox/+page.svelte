<script lang="ts">
    import { Bomb, Ellipsis, Flag, Shield, Trash } from "lucide-svelte";
    import { onMount } from "svelte";

    import { active_page } from "$lib/stores.js";
    active_page.set("inbox");

    import Question from "$lib/components/Question.svelte";
    import Dropdown from "$lib/components/Dropdown.svelte";
    import MoreResponseOptions from "$lib/components/MoreResponseOptions.svelte";
    import { Option } from "$lib/classes/Option";

    const { data } = $props();
    const lang = data.lang;
    const page = data.data;
    const config = data.config;
    const user = Option.from(data.user);

    const { is_helper, unread } = page;

    onMount(() => {
        setTimeout(() => {
            trigger("questions:carp");
        }, 150);
    });
</script>

<svelte:head>
    <title>Inbox - {config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

<article>
    <main class="flex flex-col gap-2">
        {#if is_helper}
            <div class="pillmenu convertible shadow">
                <a href="/inbox" class="active"><span>My Inbox</span></a>
                <a href="/inbox/audit"><span>Audit Log</span></a>
                <a href="/inbox/reports"><span>Reports</span></a>
            </div>
        {/if}

        {#if unread.length === 0}
            <div class="markdown-alert-warning">
                <span>{lang["general:text.no_results"]}</span>
            </div>
        {:else}
            <div class="w-full flex justify-between">
                <div></div>
                <button
                    class="red"
                    onclick={async () => {
                        if (
                            !(await trigger("app:confirm", [
                                "Are you sure you want to do this? This will delete every question currently in your inbox permanently."
                            ]))
                        ) {
                            return;
                        }

                        fetch(`/api/v1/questions/inbox/me/clear`, {
                            method: "POST"
                        })
                            .then((res) => res.json())
                            .then((res) => {
                                trigger("app:toast", [
                                    res.success ? "success" : "error",
                                    res.success ? "Inbox cleared!" : res.message
                                ]);
                            });
                    }}
                >
                    <Bomb class="icon" />
                    {lang["general:action.clear"]}
                </button>
            </div>

            {#each unread as question}
                <div class="card card-nest" id="question:{question.id}">
                    {#snippet actions()}
                        <Dropdown>
                            <button class="camo">
                                <Ellipsis class="icon" />
                            </button>

                            <div class="inner shadow-md">
                                <b class="title">Manage</b>
                                <button
                                    onclick={() => {
                                        trigger("questions:delete", [
                                            question.id
                                        ]);
                                    }}
                                    class="red"
                                >
                                    <Trash class="icon" />
                                    {lang["general:action.delete"]}
                                </button>

                                <button
                                    onclick={() => {
                                        trigger("questions:ipblock", [
                                            question.id
                                        ]);
                                    }}
                                >
                                    <Shield class="icon" />
                                    {lang["general:action.ip_block"]}
                                </button>

                                <button
                                    onclick={() => {
                                        trigger("reports:bootstrap", [
                                            "questions",
                                            question.id
                                        ]);
                                    }}
                                >
                                    <Flag class="icon" />
                                    {lang["general:action.report"]}
                                </button>
                            </div>
                        </Dropdown>
                    {/snippet}

                    <Question {question} {actions} />

                    <div class="card">
                        <form
                            class="flex flex-col gap-2"
                            onsubmit={(e) => {
                                e.preventDefault();
                                (e.target as any)
                                    .querySelector("button")
                                    .setAttribute("disabled", "true");

                                trigger("responses:create", [
                                    question.id,
                                    (e.target as any).content.value,
                                    (e.target as any).tags.value,
                                    (e.target as any).warning.value,
                                    (e.target as any).reply.value,
                                    (e.target as any).unlisted.checked
                                ]).then(() => {
                                    // reset if successful
                                    (e.target as any).reset();
                                    (e.target as any)
                                        .querySelector("button")
                                        .removeAttribute("disabled");
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
                                id="content-{question.id}"
                                data-hook="counter"
                            ></textarea>

                            <MoreResponseOptions {lang} profile={user} />

                            <div class="flex justify-between w-full gap-1">
                                <span
                                    id="content-{question.id}:counter"
                                    class="notification item"
                                ></span>
                                <button class="primary bold">
                                    {lang["general:form.submit"]}
                                </button>
                            </div>
                        </form>
                    </div>
                </div>
            {/each}
        {/if}
    </main>
</article>
