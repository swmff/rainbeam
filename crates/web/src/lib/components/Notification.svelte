<script lang="ts">
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { Notification } from "$lib/bindings/Notification";
    import { render_markdown } from "$lib/helpers";
    import { BellMinus, ExternalLink } from "lucide-svelte";

    const {
        notif,
        lang,
        show_mark_as_read = true
    }: {
        notif: Notification;
        lang: LangFile["data"];
        show_mark_as_read: boolean;
    } = $props();
</script>

<div class="card-nest w-full shadow" id="notif:{notif.id}">
    <div class="card flex flex-wrap justify-between gap-2">
        <span class="notif_title" data-do="notif_title" data-id={notif.id}>
            {@html render_markdown(notif.title)}
        </span>

        <span class="notif_timestamp date">{notif.timestamp}</span>
    </div>

    <div class="card flex flex-col gap-2">
        <div class="notif_content" data-hook="long">
            {@html render_markdown(notif.content)}
        </div>

        <div class="flex gap-2">
            {#if notif.address}
                <a
                    class="button primary bold"
                    href={notif.address}
                    onclick={() => {
                        trigger("notifications:onopen", [notif.id]);
                    }}
                    data-do="notification"
                >
                    <ExternalLink class="icon" />
                    {lang["general:link.open"]}
                </a>
            {/if}

            {#if show_mark_as_read}
                <button
                    class="button secondary bold"
                    onclick={() => {
                        trigger("notifications:delete", [notif.id]);
                    }}
                >
                    <BellMinus class="icon" />{lang["general:action.delete"]}
                </button>
            {/if}
        </div>
    </div>
</div>
