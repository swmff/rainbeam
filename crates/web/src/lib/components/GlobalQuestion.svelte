<script lang="ts">
    import type { Option } from "$lib/classes/Option";
    import type { Question } from "$lib/bindings/Question";
    import { Flag, Heart, Reply, Trash } from "lucide-svelte";
    import type { Profile } from "$lib/bindings/Profile";
    import QuestionComponent from "$lib/components/Question.svelte";

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
</script>

<div class="card-nest w-full" id="question:{question.id}">
    <QuestionComponent {question} actions={null} />

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
            {:else}
                <button
                    class="red"
                    onclick={() => {
                        trigger("questions:delete", [question.id]);
                    }}
                >
                    <Trash class="icon" />
                </button>
            {/if}
        {/if}
    </div>
</div>
