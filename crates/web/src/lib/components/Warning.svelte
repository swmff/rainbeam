<script lang="ts">
    import type { Warning } from "$lib/bindings/Warning";
    import { render_markdown } from "$lib/helpers";
    import type { Option } from "$lib/classes/Option";
    import type { Profile } from "$lib/bindings/Profile";

    import { Copy, Ellipsis, Trash } from "lucide-svelte";
    import Dropdown from "./Dropdown.svelte";

    const { warning, profile }: { warning: Warning; profile: Option<Profile> } = $props();
</script>

<div class="card-nest w-full shadow" id="warning:{warning.id}">
    <div class="card flex flex-col gap-1">
        <div class="flex justify-between gap-1 warning_title">
            <div class="footernav items-center">
                <b class="flex items-center gap-2 item">
                    <img
                        title="{warning.moderator.username}'s avatar"
                        src="/api/v0/auth/profile/{warning.moderator.id}/avatar"
                        alt=""
                        class="avatar"
                        loading="lazy"
                        style="--size: 20px"
                    />

                    <a href="/@{warning.moderator.username}" style="color: inherit">
                        {warning.moderator.username}
                    </a>
                </b>

                <span class="date item">{warning.timestamp}</span>
            </div>

            <div class="flex gap-2">
                <!-- options -->
                <Dropdown>
                    <button class="icon-only camo">
                        <Ellipsis class="icon" />
                    </button>

                    <div class="inner w-content">
                        <!-- actions for warning moderator ONLY -->
                        {#if profile.is_some()}
                            {@const user = profile.unwrap()}
                            {#if user.id === warning.moderator.id}
                                <b class="title">Manage</b>
                                <button
                                    onclick={() => {
                                        trigger("account_warnings:delete", [warning.id]);
                                    }}
                                    class="red"
                                >
                                    <Trash class="icon" /> Delete
                                </button>
                            {/if}
                        {/if}
                        <!-- actions for everybody -->
                        <b class="title">Tools</b>
                        <button
                            onclick={() => {
                                trigger("app:copy_text", [warning.id]);
                            }}
                        >
                            <Copy class="icon" /> Copy ID
                        </button>
                    </div>
                </Dropdown>
            </div>
        </div>
    </div>

    <div class="card warning_content">
        {@html render_markdown(warning.content)}
    </div>
</div>
