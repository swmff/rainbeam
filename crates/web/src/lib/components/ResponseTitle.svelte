<script lang="ts">
    import type { QuestionResponse } from "$lib/bindings/QuestionResponse";
    import type { Question } from "$lib/bindings/Question";
    import {
        Copy,
        Ellipsis,
        ExternalLink,
        Flag,
        Heart,
        MessageCircle,
        Pen,
        Pin,
        PinOff,
        Quote,
        Repeat2,
        Reply,
        Rocket,
        Tag,
        Trash,
        Undo2
    } from "lucide-svelte";

    import Dropdown from "./Dropdown.svelte";
    import type { Option } from "$lib/classes/Option";
    import type { Profile } from "$lib/bindings/Profile";
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { CleanConfig } from "$lib/db";

    const {
        res,
        is_helper,
        is_pinned,
        show_pin_button,
        do_render_nested = true,
        show_comments = true,
        profile,
        lang,
        config
    }: {
        res: [Question, QuestionResponse, number, number];
        is_helper: boolean;
        is_pinned: boolean;
        show_pin_button: boolean;
        do_render_nested: boolean;
        show_comments: boolean;
        profile: Option<Profile>;
        lang: LangFile["data"];
        config: CleanConfig;
    } = $props();

    const [question, response, comment_count, reaction_count] = res;
</script>

<div
    class="flex justify-between items-center flex-collapse sm:items-start gap-1 response_title"
>
    <div class="footernav items-center flex-wrap">
        <b class="flex items-center gap-2">
            <img
                title="{response.author.username}'s avatar"
                src="/api/v0/auth/profile/{response.author.id}/avatar"
                alt=""
                class="avatar"
                loading="lazy"
                style="--size: 20px"
            />

            <a
                href="/@{response.author.username}"
                style="color: inherit"
                class="username short"
            >
                {#if response.author.metadata.kv["sparkler:display_name"]}
                    {response.author.metadata.kv["sparkler:display_name"]}
                {:else}
                    {response.author.username}
                {/if}
            </a>
        </b>

        <span class="flex fade item">
            {#if response.edited !== BigInt(0) && response.edited !== response.timestamp}
                <span class="date item">{response.edited}</span>
                <sup title="Edited">*</sup>
            {:else}
                <span class="date item">{response.timestamp}</span>
            {/if}
        </span>

        {#if is_pinned}
            <a
                class="item flex items-center justify-center icon-only button primary small"
                title="This question/response is pinned"
                href="/@{response.author.username}"
            >
                <Pin class="icon" />
            </a>
        {/if}

        {#if response.context.circle}
            <a
                class="button item camo icon-only small avatar"
                href="/+g/{response.context.circle}"
                title="Posted in circle"
                style="
                            --size: 20px;
                            background: url('/api/v1/circles/{response.context
                    .circle}/avatar');
                            border-radius: var(--radius);
                            background-size: cover;
                            padding: 0 !important;
                            width: var(--size) !important;
                            height: var(--size) !important;
                            min-width: var(--size) !important;
                            min-height: var(--size) !important;
                        "
                aria-label="Posted in circle"
            >
            </a>
        {/if}
    </div>

    {#if do_render_nested}
        <div class="flex justify-between gap-2 sm:w-full actions_bar">
            <!-- reactions -->
            <button
                title="{reaction_count} reactions"
                class="camo"
                onclick={(event) => {
                    trigger("reactions:toggle", [
                        response.id,
                        "Response",
                        event.target
                    ]);
                }}
                data-hook="check_reaction"
                data-hook-arg_id={response.id}
            >
                <Heart class="icon" />
                {#if reaction_count > 0}
                    <span class="notification camo">{reaction_count}</span>
                {/if}
            </button>

            {#if show_comments}
                <a
                    href="/@{response.author.username}/r/{response.id}"
                    title="{comment_count} comments"
                    class="button camo"
                >
                    <MessageCircle class="icon" />
                    {#if comment_count > 0}
                        <span class="notification camo">{comment_count}</span>
                    {/if}
                </a>
            {/if}

            <!-- quote -->
            <Dropdown>
                <button class="w-full camo" title="Quote">
                    <Repeat2 class="icon" />
                </button>

                <div class="inner w-content">
                    {#if profile.is_some()}
                        <button
                            onclick={() => {
                                trigger("responses:create", [
                                    "0",
                                    `✨ Boost\n/+r/${response.id}`
                                ]).then((p: any) => {
                                    p.success
                                        ? (window.location.href = `/response/${p.payload.id}`)
                                        : "";
                                });
                            }}
                        >
                            <Rocket class="icon" />
                            {lang["response_title.html:action.boost"]}
                        </button>

                        <a
                            href="/intents/post?reply={response.id}&title=Quote%20post"
                        >
                            <Quote class="icon" />
                            {lang["response_title.html:action.quote"]}
                        </a>
                    {/if}

                    <a
                        href="/@{response.author
                            .username}?reply_intent={response.id}#top"
                        data-turbo="false"
                        target="_blank"
                    >
                        <Reply class="icon" />
                        {lang["response_title.html:action.ask_about_this"]}
                    </a>
                </div>
            </Dropdown>

            <!-- options -->
            <Dropdown>
                <button class="w-full camo" title="More">
                    <Ellipsis class="icon" />
                </button>

                <div class="inner w-content">
                    <b class="title">Sharing</b>

                    <button
                        onclick={(event) => {
                            trigger("app:copy_text", [
                                trigger("responses:gen_share", [
                                    event.target,
                                    response.id,
                                    280
                                ])
                            ]);
                        }}
                    >
                        <Copy class="icon" />
                        {lang["response_title.html:action.copy_to_clipboard"]}
                    </button>

                    <button
                        onclick={(event) => {
                            trigger("app:intent_twitter", [
                                trigger("responses:gen_share", [
                                    event.target,
                                    response.id,
                                    280,
                                    false
                                ]),
                                `${config.host}/+r/${response.id}`
                            ]);
                        }}
                    >
                        <ExternalLink class="icon" /> Twitter
                    </button>

                    <button
                        onclick={(event) => {
                            trigger("app:intent_bluesky", [
                                trigger("responses:gen_share", [
                                    event.target,
                                    response.id,
                                    280,
                                    false
                                ]),
                                `${config.host}/+r/${response.id}`
                            ]);
                        }}
                    >
                        <ExternalLink class="icon" /> Bluesky
                    </button>

                    <button
                        onclick={() => {
                            trigger("app:copy_text", [
                                `${config.host}/+r/${response.id}`
                            ]);
                        }}
                    >
                        <Copy class="icon" />
                        {lang["general:action.copy_link"]}
                    </button>

                    {#if profile.is_some()}
                        {@const user = profile.unwrap() as Profile}
                        {#if user.id == response.author.id}
                            <!-- actions for the profile owner only -->
                            <b class="title">Manage</b>

                            <!-- pin -->
                            {#if show_pin_button == true && is_pinned == false}
                                <button
                                    onclick={() => {
                                        (globalThis as any).pin_response(
                                            response.id
                                        );
                                    }}
                                >
                                    <Pin class="icon" />
                                    {lang["response_title.html:action.pin"]}
                                </button>
                            {:else if show_pin_button}
                                <button
                                    onclick={() => {
                                        (globalThis as any).unpin_response(
                                            response.id
                                        );
                                    }}
                                >
                                    <PinOff class="icon" />
                                    {lang["response_title.html:action.unpin"]}
                                </button>
                            {/if}

                            <!-- ... -->
                            <a
                                href="/@{response.author
                                    .username}/r/{response.id}#/edit"
                            >
                                <Pen class="icon" />
                                {lang["general:action.edit"]}
                            </a>

                            <a
                                href="/@{response.author
                                    .username}/r/{response.id}#/tags"
                            >
                                <Tag class="icon" />
                                {lang["response_title.html:action.edit_tags"]}
                            </a>

                            {#if response.context.is_post == false}
                                <button
                                    onclick={() => {
                                        trigger("responses:unsend", [
                                            response.id
                                        ]);
                                    }}
                                    class="red"
                                >
                                    <Undo2 class="icon" />
                                    {lang[
                                        "response_title.html:action.return_to_inbox"
                                    ]}
                                </button>
                            {/if}

                            <button
                                onclick={() => {
                                    trigger("responses:delete", [response.id]);
                                }}
                                class="red"
                            >
                                <Trash class="icon" />
                                {lang["response_title.html:action.delete_all"]}
                            </button>
                        {/if}
                    {/if}

                    <!-- actions for everybody -->
                    <b class="title">Tools</b>
                    <button
                        onclick={() => {
                            trigger("app:copy_text", [response.id]);
                        }}
                    >
                        <Copy class="icon" />{lang["general:action.copy_id"]}
                    </button>

                    <a href="/@{response.author.username}/r/{response.id}">
                        <ExternalLink class="icon" />
                        {lang["general:link.open"]}
                    </a>

                    {#if is_helper}
                        <a href="/@{question.author.username}/q/{question.id}">
                            <ExternalLink class="icon" />
                            {lang["response_title.html:link.open_question"]}
                        </a>
                    {/if}

                    {#if profile.is_some()}
                        {@const user = profile.unwrap() as Profile}
                        {#if user.id == response.author.id}
                            <!-- actions for users that ARE NOT the author -->
                            <button
                                onclick={() => {
                                    trigger("reports:bootstrap", [
                                        "responses",
                                        response.id
                                    ]);
                                }}
                            >
                                <Flag class="icon" />
                                {lang["general:action.report"]}
                            </button>
                        {/if}
                        {#if is_helper}
                            <b class="title">Mod</b>
                            <button
                                onclick={() => {
                                    trigger("responses:delete", [response.id]);
                                }}
                                class="red"
                            >
                                <Trash class="icon" />
                                {lang["general:action.delete"]}
                            </button>
                        {/if}
                    {/if}
                </div>
            </Dropdown>
        </div>
    {/if}
</div>
