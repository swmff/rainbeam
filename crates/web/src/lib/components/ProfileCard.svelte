<script lang="ts">
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { Profile } from "$lib/bindings/Profile";
    import type { Option } from "$lib/classes/Option";
    import UserNote from "./UserNote.svelte";

    const { user, current_profile, lang }: { user: Profile; current_profile: Option<Profile>; lang: LangFile["data"] } =
        $props();
</script>

<div class="card-nest w-full" id="card:{user.id}">
    <div class="card" style="padding: 0">
        <a href="/@{user.username}" data-sveltekit-reload>
            <img
                title="{user.username}'s banner"
                src="/api/v0/auth/profile/{user.id}/banner"
                alt=""
                class="shadow round"
                style="
                    width: 100%;
                    min-height: 80px;
                    max-height: 80px;
                    object-fit: cover;
                    border-bottom-left-radius: 0 !important;
                    border-bottom-right-radius: 0 !important;
                "
            />
        </a>
    </div>

    <div class="card flex gap-2">
        <a href="/@{user.username}" data-sveltekit-reload>
            <img
                title="{user.username}'s avatar"
                src="/api/v0/auth/profile/{user.id}/avatar"
                alt=""
                class="avatar shadow-md"
                style="--size: 80px; margin: -50px 0.5rem 0"
            />
        </a>

        <div class="flex items-center gap-2">
            <h3 class="no-margin">
                <a href="/@{user.username}" data-sveltekit-reload>
                    <!-- prettier-ignore -->
                    {#if user.metadata.kv["sparkler:display_name"]}
                        {user.metadata.kv["sparkler:display_name"]}
                    {:else}
                         {user.username }
                    {/if}
                </a>
            </h3>

            <UserNote {user} use_static={true} {current_profile} {lang} />
        </div>
    </div>
</div>
