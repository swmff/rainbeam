<script lang="ts">
    import type { Option } from "$lib/classes/Option";
    import type { QuestionResponse } from "$lib/bindings/QuestionResponse";
    import { anonymous_tag, render_markdown } from "$lib/helpers";
    import type { Profile } from "$lib/bindings/Profile";
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { CleanConfig } from "$lib/db";
    import {
        Copy,
        Ellipsis,
        ExternalLink,
        Flag,
        Heart,
        MessageCircle,
        Pen,
        Shield,
        Trash
    } from "lucide-svelte";
    import Dropdown from "./Dropdown.svelte";
    import type { ResponseComment } from "$lib/bindings/ResponseComment";

    const {
        com,
        is_powerful,
        is_helper,
        profile,
        lang,
        config,
        show_replies = true
    }: {
        com: [QuestionResponse, ResponseComment, number, number];
        is_powerful: boolean;
        is_helper: boolean;
        profile: Option<Profile>;
        lang: LangFile["data"];
        config: CleanConfig;
        show_replies: boolean;
    } = $props();

    const [response, comment, reply_count, reaction_count] = com;
    const author_tag = anonymous_tag(comment.author.id);
</script>

<div class="card flex flex-col gap-2 comment_body" id="comment:{comment.id}">
    <div class="footernav items-center comment_title">
        <b class="flex items-center gap-2 item">
            {#if author_tag[0] === false}
                {@const display_name =
                    comment.author.metadata.kv["sparkler:display_name"]}

                <img
                    title="{comment.author.username}'s avatar"
                    src="/api/v0/auth/profile/{comment.author.id}/avatar"
                    alt=""
                    class="avatar"
                    loading="lazy"
                    style="--size: 24px"
                />

                <a
                    href="/@{comment.author.username}"
                    style="color: inherit"
                    class="username short"
                >
                    {#if display_name}
                        {#if display_name.trim()}
                            {display_name.trim()}
                        {:else}
                            {comment.author.username}
                        {/if}
                    {:else}
                        {comment.author.username}
                    {/if}
                </a>
            {:else}
                <!-- default avatar, setting not set OR blank or unsafe -->
                <img
                    title="anonymous' avatar"
                    src="/images/default-avatar.svg"
                    alt=""
                    class="avatar"
                    loading="lazy"
                    style="--size: 24px"
                />

                <span>anonymous</span>
            {/if}

            {#if is_powerful}
                {#if author_tag[0]}
                    <a href="/+u/{author_tag[1]}" class="notification">
                        {#if author_tag[1].length >= 10}
                            {author_tag[1].substring(0, 10)}
                        {:else}
                            {author_tag[1]}
                        {/if}
                    </a>
                {/if}
            {/if}
        </b>

        <span class="flex fade item">
            {#if comment.edited !== BigInt(0) && comment.edited !== comment.timestamp}
                <span class="date item">{comment.edited}</span>
                <sup title="Edited">*</sup>
            {:else}
                <span class="date item">{comment.timestamp}</span>
            {/if}
        </span>
    </div>

    <span class="comment_content" data-hook="long">
        {@html render_markdown(comment.content)}
    </span>

    <div class="flex w-full gap-2 actions_bar w-full justify-between">
        <div class="flex gap-2 sm:contents">
            <!-- reactions -->
            <button
                title="{reaction_count} reactions"
                class="circle camo sm:w-full"
                onclick={(event) => {
                    trigger("reactions:toggle", [
                        comment.id,
                        "Comment",
                        event.target
                    ]);
                }}
                data-hook="check_reaction"
                data-hook-arg_id={comment.id}
            >
                <Heart class="icon" />
                {#if reaction_count}
                    <span class="notification camo">{reaction_count}</span>
                {/if}
            </button>

            <!-- replies -->
            {#if show_replies}
                <a
                    href="/@{comment.author.username}/c/{comment.id}"
                    title="{reply_count} replies"
                    class="circle button camo sm:w-full"
                >
                    <MessageCircle class="icon" />
                    {#if reply_count}
                        <span class="notification camo">{reply_count}</span>
                    {/if}
                </a>
            {/if}
        </div>

        <!-- options -->
        <Dropdown classname="sm:w-full">
            <button class="circle camo w-full">
                <Ellipsis class="icon" />
            </button>

            <div class="inner w-content">
                <b class="title">Sharing</b>

                <button
                    onclick={() => {
                        trigger("app:copy_text", [
                            `${config.host}/+c/${comment.id}`
                        ]);
                    }}
                >
                    <Copy class="icon" />
                    {lang["general:action.copy_link"]}
                </button>

                {#if profile.is_some()}
                    {@const user = profile.unwrap()}
                    {#if user.id == comment.author.id || user.id == response.author.id}
                        <!-- actions for the comment author/response author only -->
                        <b class="title">Manage</b>

                        <a
                            href="/@{comment.author
                                .username}/c/{comment.id}#/edit"
                        >
                            <Pen class="icon" />
                            {lang["general:action.edit"]}
                        </a>

                        <button
                            onclick={() => {
                                trigger("comments:delete", [comment.id]);
                            }}
                            class="red"
                        >
                            <Trash class="icon" />
                            {lang["general:action.delete"]}
                        </button>

                        <button
                            onclick={() => {
                                trigger("comments:ipblock", [comment.id]);
                            }}
                        >
                            <Shield class="icon" />
                            {lang["general:action.ip_block"]}
                        </button>
                    {/if}
                {/if}
                <!-- actions for everybody -->
                <b class="title">Tools</b>
                <button
                    onclick={() => {
                        trigger("app:copy_text", [comment.id]);
                    }}
                >
                    <Copy class="icon" />
                    {lang["general:action.copy_id"]}
                </button>

                <a href="/@{comment.author.username}/c/{comment.id}">
                    <ExternalLink class="icon" />
                    {lang["general:link.open"]}
                </a>

                {#if profile.is_some()}
                    {@const user = profile.unwrap()}
                    {#if user.id != comment.author.id}
                        <!-- actions for users that ARE NOT the author -->
                        <a
                            href="javascript:trigger('reports:bootstrap', ['comments', '{comment.id}'])"
                        >
                            <Flag class="icon" />
                            {lang["general:action.report"]}
                        </a>
                    {/if}
                    {#if is_helper}
                        <b class="title">Mod</b>
                        <button
                            onclick={() => {
                                trigger("comments:delete", [comment.id]);
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
</div>
