<script lang="ts">
    import { ChevronRight } from "lucide-svelte";

    import type { Chat } from "$lib/bindings/Chat";
    import type { Profile } from "$lib/bindings/Profile";

    const { chat }: { chat: [Chat, Array<Profile>] } = $props();
</script>

<a href="/chats/{chat[0].id}" class="card w-fill flex items-center justify-between" data-turbo="false">
    <div class="flex flex-col gap-1">
        <b>{chat[0].name}</b>
        <div class="footernav flex-wrap">
            {#each chat[1] as user}
                <span class="item flex items-center gap-2">
                    <img
                        title="{user.username}}'s avatar"
                        src="/api/v0/auth/profile/{user.id}/avatar"
                        alt="@{user.username}"
                        class="avatar"
                        loading="lazy"
                        style="--size: 20px"
                    />
                    {user.username}
                </span>
            {/each}
        </div>
    </div>

    <button class="primary icon-only">
        <ChevronRight class="icon" />
    </button>
</a>
