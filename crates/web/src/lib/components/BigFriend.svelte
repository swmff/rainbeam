<script lang="ts">
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { Profile } from "$lib/bindings/Profile";
    import type { Option } from "$lib/classes/Option";
    import UserNote from "./UserNote.svelte";

    const {
        user,
        profile,
        lang
    }: { user: Profile; profile: Option<Profile>; lang: LangFile["data"] } =
        $props();
</script>

<div style="margin-right: 1rem; flex-shrink: 0">
    <div style="position: relative">
        <a href="/@{user.username}">
            <img
                title="{user.username}'s avatar"
                src="/api/v0/auth/profile/{user.id}/avatar"
                alt="@{user.username}"
                class="avatar"
                loading="lazy"
                style="--size: 50px"
            />
        </a>

        <UserNote {user} current_profile={profile} {lang} />
    </div>
</div>
