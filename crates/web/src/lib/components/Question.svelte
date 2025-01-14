<script lang="ts">
    import type { Question } from "$lib/bindings/Question";
    import { anonymous_tag, render_markdown } from "$lib/helpers";

    const {
        question
    }: {
        question: Question;
    } = $props();

    const author_tag = anonymous_tag(question.author.id);
</script>

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
                        question.author.metadata.kv["sparkler:display_name"]}

                    <a
                        href="/@{question.author.username}"
                        style="color: inherit"
                        class="username short"
                    >
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
