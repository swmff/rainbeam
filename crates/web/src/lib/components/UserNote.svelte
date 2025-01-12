<script lang="ts">
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { Profile } from "$lib/bindings/Profile";
    import type { Option } from "$lib/classes/Option";
    import {
        CircleUserRound,
        Ellipsis,
        Flag,
        MessageCirclePlus,
        Pen,
        Trash,
        X
    } from "lucide-svelte";
    import Dropdown from "./Dropdown.svelte";

    const {
        user,
        current_profile,
        lang
    }: {
        user: Profile;
        current_profile: Option<Profile>;
        lang: LangFile["data"];
    } = $props();
</script>

{#if current_profile.is_some()}
    {@const profile = current_profile.unwrap()}

    {@const note = user.metadata.kv["sparkler:status_note"]}
    {@const emoji = user.metadata.kv["sparkler:status_emoji"]}

    {#if note || profile.id === user.id}
        {#if note || profile.id === user.id}
            <button
                class="status_note primary"
                title="View note"
                onclick={() => {
                    (
                        document.getElementById(
                            `status:${user.id}`
                        ) as HTMLDialogElement
                    ).showModal();
                }}
                style="border: solid 2px var(--color-surface) !important"
            >
                <!-- prettier-ignore -->
                {#if emoji}
                    {emoji}
                {:else}
                    💭
                {/if}
            </button>

            <dialog id="status:{user.id}">
                <div class="inner" style="min-height: 250px">
                    <div class="w-full flex justify-between items-center gap-2">
                        <b>{user.username}</b>
                        <div class="flex gap-2">
                            {#if profile.id === user.id}
                                <a
                                    href="/settings?note"
                                    class="button camo icon-only"
                                    title="Edit note"
                                    target="_blank"
                                >
                                    <Pen class="icon" />
                                </a>
                            {/if}

                            <Dropdown>
                                <button class="camo title">
                                    <Ellipsis class="icon" />
                                </button>

                                <div class="inner shadow-md w-content">
                                    {#if profile.id === user.id}
                                        <a
                                            href="/settings?note_clear"
                                            target="_blank"
                                            class="red"
                                        >
                                            <Trash class="icon" />
                                            {lang["general:action.clear"]}
                                        </a>
                                    {/if}

                                    <b class="title">Actions</b>

                                    <a href="/@{user.username}">
                                        <CircleUserRound class="icon" />
                                        {lang["general:link.show_profile"]}
                                    </a>

                                    <button
                                        onclick={() => {
                                            trigger("chats:create", [user.id]);
                                        }}
                                    >
                                        <MessageCirclePlus class="icon" />
                                        {lang["general:link.chat"]}
                                    </button>

                                    <button
                                        onclick={() => {
                                            trigger("reports:bootstrap", [
                                                "profiles",
                                                user.username
                                            ]);
                                        }}
                                    >
                                        <Flag class="icon" />
                                        {lang["general:action.report"]}
                                    </button>
                                </div>
                            </Dropdown>

                            <button
                                class="bold red camo icon-only"
                                onclick={() => {
                                    (
                                        document.getElementById(
                                            `status:${user.id}`
                                        ) as HTMLDialogElement
                                    ).close();
                                }}
                                type="button"
                                title="Close"
                            >
                                <X class="icon" />
                            </button>
                        </div>
                    </div>

                    <hr class="flipped" />
                    <span>{note}</span>
                </div>
            </dialog>
        {/if}
    {/if}
{/if}
