<script lang="ts">
    import MoreResponseOptions from "$lib/components/MoreResponseOptions.svelte";
    import { Option } from "$lib/classes/Option.js";
    import { onMount } from "svelte";

    const { data } = $props();
    const lang = data.lang;
    const user = Option.from(data.user);

    onMount(() => {
        setTimeout(() => {
            (globalThis as any).__init();

            use("codemirror", (codemirror: any) => {
                codemirror.create_editor(
                    document.getElementById("compose_editor"),
                    "",
                    "Tell your friends what's on your mind...",
                    "compose_editor_"
                );
            });
        }, 100);
    });
</script>

{#if user.is_some()}
    <article>
        <main class="flex flex-col gap-2">
            <script
                src="https://unpkg.com/codemirror@5.39.2/lib/codemirror.js"
            ></script>
            <script
                src="https://unpkg.com/codemirror@5.39.2/addon/display/placeholder.js"
            ></script>
            <script
                src="https://unpkg.com/codemirror@5.39.2/mode/markdown/markdown.js"
            ></script>

            <link
                rel="stylesheet"
                href="https://unpkg.com/codemirror@5.39.2/lib/codemirror.css"
            />

            <div id="create_post_intent" class="flex flex-col gap-2">
                <b class="title">Create post</b>
                <div id="partial"></div>
                <div class="card w-full">
                    <form
                        id="post_form"
                        class="flex flex-col gap-2"
                        onsubmit={(e) => {
                            e.preventDefault();
                            (e.target as any)
                                .querySelector("button")
                                .setAttribute("disabled", "true");

                            trigger("responses:create", [
                                "0", // posts use "0" as their question ID
                                (globalThis as any).compose_editor_.getValue(),
                                (e.target as any).tags.value,
                                (e.target as any).warning.value,
                                (e.target as any).reply.value,
                                (e.target as any).unlisted.checked,
                                (e.target as any).circle.value
                            ]).then((res: any) => {
                                // reset if successful
                                (e.target as any).reset();
                                (e.target as any)
                                    .querySelector("button")
                                    .removeAttribute("disabled");

                                (globalThis as any).compose_editor_.setValue(
                                    ""
                                );
                                (
                                    globalThis as any
                                ).compose_editor_.clearHistory();

                                // open post
                                window.location.href = `/response/${res.payload.id}`;
                            });
                        }}
                    >
                        <div id="compose_editor" class="post_editor"></div>
                        <MoreResponseOptions {lang} profile={user} />

                        <hr style="margin-top 0.75rem !important" />
                        <div class="flex justify-between w-full gap-1">
                            <div></div>
                            <div class="flex gap-2">
                                <button class="primary bold"
                                    >{lang["general:link.post"]}</button
                                >

                                <button
                                    type="button"
                                    class="bold"
                                    onclick={() => {
                                        (globalThis as any).go_back_confirm();
                                    }}
                                >
                                    {lang["general:dialog.cancel"]}
                                </button>
                            </div>
                        </div>
                    </form>
                </div>
            </div>

            <script>
                const search = new URLSearchParams(window.location.search);

                if (search.get("reply")) {
                    const reply = search.get("reply");

                    document
                        .querySelector("#create_post_intent details")
                        .setAttribute("open", "true");

                    document.querySelector(
                        '#create_post_intent [name="reply"]'
                    ).value = reply;

                    document.getElementById("partial").innerHTML =
                        `<include-partial
                            src="/_components/response?id=${reply}&do_render_nested=false"
                            uses="app:clean_date_codes,app:link_filter,app:hook.alt"
                        ></include-partial>`;
                }

                if (search.get("circle")) {
                    const circle = search.get("circle");
                    search.set("title", "Create post in circle");

                    document
                        .querySelector("#create_post_intent details")
                        .setAttribute("open", "true");

                    document.querySelector(
                        '#create_post_intent [name="circle"]'
                    ).value = circle;
                }

                if (search.get("title")) {
                    const title = search.get("title");

                    document.querySelector(
                        "#create_post_intent b.title"
                    ).innerText = title;
                }

                async function are_you_sure() {
                    return await trigger("app:confirm", [
                        "Are you sure you would like to leave this page?"
                    ]);
                }

                globalThis.go_back_confirm = async () => {
                    if (await are_you_sure()) {
                        window.history.back();
                    }
                };
            </script>
        </main>
    </article>
{/if}
