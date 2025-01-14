<script lang="ts">
    import type { Option } from "$lib/classes/Option";
    import type { Question } from "$lib/bindings/Question";
    import type { QuestionResponse } from "$lib/bindings/QuestionResponse";
    import { anonymous_tag, render_markdown } from "$lib/helpers";
    import ResponseTitle from "./ResponseTitle.svelte";
    import type { Profile } from "$lib/bindings/Profile";
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { CleanConfig } from "$lib/db";

    const {
        res,
        anonymous_avatar,
        anonymous_username,
        is_powerful,
        is_helper,
        is_pinned = false,
        show_pin_button = false,
        profile,
        lang,
        config,
        do_render_nested = true,
        show_comments = true
    }: {
        res: [Question, QuestionResponse, number, number];
        anonymous_avatar: string;
        anonymous_username: string;
        is_powerful: boolean;
        is_helper: boolean;
        is_pinned: boolean;
        show_pin_button: boolean;
        profile: Option<Profile>;
        lang: LangFile["data"];
        config: CleanConfig;
        do_render_nested: boolean;
        show_comments: boolean;
    } = $props();

    const [question, response] = res;
</script>

<div
    class="flex flex-col gap-2 w-full"
    id="response:{response.id}"
    onclick={(event) => {
        trigger("responses:click", [response.id, do_render_nested]);
    }}
    onkeydown={() => {}}
    tabindex="0"
    role="button"
>
    <div class="card-nest w-full response">
        {#if response.context.is_post == false}
            {@const author_tag = anonymous_tag(question.author.id)}
            <!-- question -->
            <div
                class="card flex flex-col gap-1 question {response.context
                    .warning
                    ? 'hidden'
                    : ''}"
            >
                <div class="flex justify-between gap-1 question_title">
                    <div class="footernav items-center">
                        <b class="flex items-center gap-2 item">
                            {#if author_tag[0] === false}
                                {@const display_name =
                                    question.author.metadata.kv[
                                        "sparkler:display_name"
                                    ]}

                                <img
                                    title="{question.author.username}'s avatar"
                                    src="/api/v0/auth/profile/{question.author
                                        .id}/avatar"
                                    alt=""
                                    class="avatar"
                                    loading="lazy"
                                    style="--size: 24px"
                                />

                                <a
                                    href="/@{question.author.username}"
                                    style="color: inherit"
                                    class="username short"
                                >
                                    <!-- prettier-ignore -->
                                    {#if display_name}
                                        {#if display_name.trim()}
                                            {display_name.trim()}
                                        {:else}
                                            {question.author.username}
                                        {/if}
                                    {:else}
                                        {question.author.username}
                                    {/if}
                                </a>
                            {:else if anonymous_avatar && !anonymous_avatar.startsWith("https://")}
                                <!-- anonymous avatar, setting set and valid -->
                                <img
                                    title="This profile's anonymous avatar"
                                    src="/api/v0/util/ext/image?img={anonymous_avatar}"
                                    alt=""
                                    class="avatar"
                                    loading="lazy"
                                    style="--size: 24px"
                                />
                            {:else}
                                <!-- default avatar, setting not set OR blank or unsafe -->
                                <img
                                    title="{question.author.username}'s avatar"
                                    src="/images/default-avatar.svg"
                                    alt=""
                                    class="avatar"
                                    loading="lazy"
                                    style="--size: 24px"
                                />
                            {/if}

                            {#if author_tag[0]}
                                <span>
                                    {#if anonymous_username}
                                        {anonymous_username}
                                    {:else}
                                        anonymous
                                    {/if}
                                </span>
                            {/if}

                            {#if is_powerful}
                                {#if author_tag[0]}
                                    <a
                                        href="/+u/{author_tag[1]}"
                                        class="notification"
                                    >
                                        {#if author_tag[1].length >= 10}
                                            {author_tag[1].substring(0, 10)}
                                        {:else}
                                            {author_tag[1]}
                                        {/if}
                                    </a>
                                {/if}
                            {/if}
                        </b>

                        <span class="date item fade">{question.timestamp}</span>

                        {#if question.recipient.id === "@"}
                            <a
                                class="item primary"
                                href="/question/{question.id}"
                                title="Global question"
                            >
                                +
                            </a>
                        {/if}
                    </div>
                </div>

                <span
                    class="question_content"
                    data-hook="long"
                    data-hook-arg="lowered"
                >
                    <p style="display: none">{question.context.media}</p>
                    {@html render_markdown(question.content)}

                    {#if response.reply && response.context.is_post === false && do_render_nested === true}
                        <include-partial
                            src="/_components/response?id={response.reply}&do_render_nested=false"
                            uses="app:clean_date_codes,app:link_filter,app:hook.alt"
                            data-click="trigger('responses:click', ['{response.reply}', false]);"
                        ></include-partial>
                    {/if}
                </span>
            </div>
        {:else}
            <div class="card" style="display: none"></div>
        {/if}

        <div class="card flex flex-col gap-1 response_body">
            {#if response.context.is_post == true}
                <ResponseTitle
                    {res}
                    {is_helper}
                    {is_pinned}
                    {show_pin_button}
                    {profile}
                    {lang}
                    {do_render_nested}
                    {show_comments}
                    {config}
                />
            {/if}

            <span
                class="response_content {response.context.warning
                    ? 'hidden'
                    : ''}"
                data-hook="long"
            >
                {@html render_markdown(response.content)}

                {#if response.reply && response.context.is_post == true && do_render_nested == true}
                    <include-partial
                        src="/_components/response?id={response.reply}&do_render_nested=false"
                        uses="app:clean_date_codes,app:link_filter,app:hook.alt"
                        data-click="trigger('responses:click', ['{response.reply}', false]);"
                    ></include-partial>
                {/if}
            </span>

            {#if response.context.warning}
                <span
                    class="response_warning markdown-alert-draft"
                    style="cursor: pointer; margin-bottom: 0"
                >
                    <div class="flex flex-col gap-4 w-full">
                        {response.context.warning}

                        <div class="flex items-center gap-4">
                            <button class="bold primary border small">
                                {lang["general:dialog.okay"]}
                            </button>

                            <span class="fade text-small">
                                {lang["response_inner.html:text.click_to_view"]}
                            </span>
                        </div>
                    </div>
                </span>
            {/if}

            {#if do_render_nested}
                <span class="response_tags flex gap-2 flex-wrap">
                    {#each response.tags as tag}
                        <a
                            href="/@{response.author.username}?tag={tag}"
                            class="tag"
                        >
                            #{tag}
                        </a>
                    {/each}
                </span>
            {/if}

            {#if response.context.is_post == false}
                <ResponseTitle
                    {res}
                    {is_helper}
                    {is_pinned}
                    {show_pin_button}
                    {profile}
                    {lang}
                    {do_render_nested}
                    {show_comments}
                    {config}
                />
            {/if}
        </div>
    </div>
</div>
