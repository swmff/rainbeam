<script lang="ts">
    import type { Option } from "$lib/classes/Option";
    import type { Question } from "$lib/bindings/Question";
    import { anonymous_tag, render_markdown } from "$lib/helpers";
    import { Flag, Globe, Heart, Reply } from "lucide-svelte";
    import type { Profile } from "$lib/bindings/Profile";

    const {
        ques,
        profile,
        show_responses = true
    }: {
        ques: [Question, number, number];
        profile: Option<Profile>;
        show_responses: boolean;
    } = $props();

    const [question, response_count, reaction_count] = ques;
    const author_tag = anonymous_tag(question.author.id);
</script>

<div class="card-nest shadow w-full" id="question:{question.id}">
    <div class="card flex flex-col gap-1">
        <div class="flex items-center justify-between gap-1 question_title">
            <div class="footernav items-center">
                <b class="flex items-center gap-2 item">
                    <img
                        title="{question.author.username}'s avatar"
                        src="/api/v0/auth/profile/{question.author.id}/avatar"
                        alt=""
                        class="avatar"
                        loading="lazy"
                        style="--size: 24px"
                    />

                    {#if author_tag[0] === false}
                        {@const display_name =
                            question.author.metadata.kv[
                                "sparkler:display_name"
                            ]}

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
                    {:else}
                        <span>anonymous</span>
                    {/if}
                </b>

                <span class="date item fade">{question.timestamp}</span>
            </div>
        </div>

        <span class="question_content" data-hook="long">
            {@html render_markdown(question.content)}
        </span>
    </div>

    <div class="card flex gap-2">
        <button
            title="{{ reaction_count }} reactions"
            onclick={() => {
                trigger("reactions:toggle", [question.id, "Question"]);
            }}
            data-hook="check_reaction"
            data-hook-arg_id={question.id}
        >
            <Heart class="icon" />
            <span class="notification camo">{reaction_count}</span>
        </button>

        {#if show_responses}
            <a
                href="/@{question.author.username}/q/{question.id}"
                class="button item"
                title="{response_count} responses"
            >
                <Reply class="icon" />
                <span class="notification camo">{response_count}</span>
            </a>
        {/if}

        {#if profile.is_none()}
            <a
                class="button"
                href="javascript:trigger('reports:bootstrap', ['questions', '{question.id}'])"
                title="Report"
            >
                <Flag class="icon" />
            </a>
        {:else}
            {@const user = profile.unwrap()}
            {#if user.id != question.author.id}
                <a
                    class="button"
                    href="javascript:trigger('reports:bootstrap', ['questions', '{question.id}'])"
                    title="Report"
                >
                    <Flag class="icon" />
                </a>
            {/if}
        {/if}
    </div>
</div>
