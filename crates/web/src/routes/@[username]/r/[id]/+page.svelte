<script lang="ts">
    import { Option } from "$lib/classes/Option.js";
    import type { Profile } from "$lib/bindings/Profile.js";
    import { onMount } from "svelte";
    import Response from "$lib/components/Response.svelte";
    import { active_page } from "$lib/stores.js";
    import Comment from "$lib/components/Comment.svelte";
    import { Check } from "lucide-svelte";

    const { data } = $props();
    active_page.set("response");

    const user = Option.from(data.user);
    const config = data.config;
    const lang = data.lang;
    const page = data.data;
    const query = data.query;

    onMount(async () => {
        setTimeout(() => {
            (globalThis as any).__init();
            (globalThis as any).post_editor_.setValue(response.content);
        }, 100);
    });

    const { question, response, comments, reactions, is_helper, is_powerful } =
        page;
</script>

<article>
    <main class="flex flex-col gap-2">
        <Response
            res={[question, response, comments.length, reactions.length]}
            anonymous_avatar={page.anonymous_avatar}
            anonymous_username={page.anonymous_username}
            is_pinned={false}
            is_powerful={page.is_powerful}
            is_helper={page.is_helper}
            show_pin_button={false}
            show_comments={false}
            profile={user}
            {config}
            {lang}
            do_render_nested={true}
        />

        <hr />
        <div class="pillmenu convertible true">
            <a href="#/comments" class="active" data-tab-button="comments"
                ><span>{lang["views:text.comments"]}</span></a
            >
            <a href="#/reactions" data-tab-button="reactions"
                ><span>{lang["views:text.reactions"]}</span></a
            >
            {#if user.is_some()}
                {@const profile = user.unwrap()}
                {#if profile.id == response.author.id}
                    <a href="#/edit" data-tab-button="edit"
                        ><span>{lang["general:action.edit"]}</span></a
                    >

                    <a href="#/tags" data-tab-button="tags"
                        ><span
                            >{lang[
                                "response_title.html:action.edit_tags"
                            ]}</span
                        ></a
                    >
                {/if}
            {/if}
        </div>

        <div data-tab="comments" class="flex flex-col gap-4">
            <div class="card-nest w-full" id="comment_field">
                <div class="card flex flex-col gap-1">Leave a comment</div>

                <div class="card">
                    <form
                        class="flex flex-col gap-2"
                        onsubmit={(e) => {
                            e.preventDefault();
                            if (e.target) {
                                trigger("comments:create", [
                                    response,
                                    (e.target as any).content.value,
                                    undefined,
                                    (e.target as any).anonymous.checked
                                ]).then(() => {
                                    (e.target as any).reset();
                                });
                            }
                        }}
                    >
                        <textarea
                            class="w-full"
                            placeholder="Type your reply!"
                            minlength="1"
                            maxlength="2048"
                            required
                            name="content"
                            id="content"
                            data-hook="counter"
                        ></textarea>

                        <div class="flex justify-between w-full gap-1">
                            <div class="flex gap-2 items-center">
                                <span
                                    id="content:counter"
                                    class="notification item"
                                ></span>

                                <div class="checkbox_container item">
                                    <input
                                        type="checkbox"
                                        name="anonymous"
                                        id="anonymous"
                                    />

                                    <label for="anonymous" class="normal">
                                        {lang["general:action.hide_your_name"]}
                                    </label>
                                </div>

                                <script>
                                    function ls_anon_check() {
                                        if (
                                            window.localStorage.getItem(
                                                "always_anon"
                                            ) === "true"
                                        ) {
                                            document.getElementById(
                                                "anonymous"
                                            ).checked = true;
                                        }
                                    }

                                    ls_anon_check();
                                </script>
                            </div>

                            <button class="primary bold"
                                >{lang["general:form.submit"]}</button
                            >
                        </div>
                    </form>
                </div>
            </div>

            {#each comments as com}
                <Comment
                    com={[response, com[0], com[1], com[2]]}
                    {is_helper}
                    {is_powerful}
                    profile={user}
                    {lang}
                    {config}
                    show_replies={true}
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

                {#if comments.length != 0}
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

        {#if user.is_some()}
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

            <div class="hidden flex flex-col gap-2" data-tab="edit">
                <form
                    class="flex flex-col gap-2 w-full card shadow"
                    onsubmit={(e) => {
                        e.preventDefault();
                        trigger("responses:edit", [
                            response.id,
                            (globalThis as any).post_editor_.getValue()
                        ]);
                    }}
                >
                    <label for="edit_content">New content</label>
                    <div id="post_editor" class="post_editor"></div>

                    <script>
                        setTimeout(() => {
                            use("codemirror", (codemirror) => {
                                codemirror.create_editor(
                                    document.getElementById("post_editor"),
                                    "",
                                    "Type your post...",
                                    "post_editor_"
                                );
                            });
                        }, 500);
                    </script>

                    <div class="flex gap-2 w-full justify-right">
                        <button class="primary bold">
                            <Check class="icon" />
                            <span>{lang["general:action.save"]}</span>
                        </button>
                    </div>
                </form>

                <form
                    class="flex flex-col gap-2 w-full card shadow"
                    onsubmit={(e) => {
                        e.preventDefault();
                        trigger("responses:edit_context_warning", [
                            response.id,
                            (e.target as any).warning.value
                        ]);
                    }}
                >
                    <label for="warning">Warning</label>

                    <textarea
                        class="w-full"
                        placeholder="Type your response warning!"
                        minlength="1"
                        maxlength="4096"
                        required
                        name="warning"
                        id="warning">{response.context.warning}</textarea
                    >

                    <div class="flex gap-2 w-full justify-right">
                        <button class="primary bold">
                            <Check class="icon" />
                            <span>{lang["general:action.save"]}</span>
                        </button>
                    </div>
                </form>
            </div>
        {/if}
    </main>
</article>
