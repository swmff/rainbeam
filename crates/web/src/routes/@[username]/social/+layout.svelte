<script lang="ts">
    const { children, data }: { children: any; data: any } = $props();
    const lang = data.lang;
    const { other, followers_count, friends_count, following_count, is_self, is_helper } = data.data;

    import { active_page } from "$lib/stores.js";
    import { ArrowLeft } from "lucide-svelte";
    let active = $state("");

    import { onMount } from "svelte";

    onMount(() => {
        active_page.subscribe((v) => {
            active = v;
        });
    });
</script>

<div class="flex flex-col gap-4">
    <a href="/@{other.username}" class="button">
        <ArrowLeft class="icon" />
        {lang["profile:link.feed"]}
    </a>

    <!-- menu -->
    <div class="pillmenu convertible shadow true">
        <a href="/@{other.username}/social/followers" class={active === "social.followers" ? "active" : ""}>
            <span>
                {lang["profile:link.followers"]}
                <b class="notification">{followers_count}</b></span
            >
        </a>

        <a href="/@{other.username}/social/following" class={active === "social.following" ? "active" : ""}>
            <span>
                {lang["profile:link.following"]}
                <b class="notification">{following_count}</b></span
            >
        </a>

        <a href="/@{other.username}/social/friends" class={active === "social.friends" ? "active" : ""}>
            <span>
                {lang["general:link.friends"]}
                <b class="notification">{friends_count}</b></span
            >
        </a>

        {#if is_self || is_helper}
            <a href="/@{other.username}/social/friends/requests" class={active === "social.requests" ? "active" : ""}
                ><span>{lang["general:link.requests"]}</span></a
            >
        {/if}

        {#if is_helper}
            <a href="/@{other.username}/social/friends/blocks" class={active === "social.blocks" ? "active" : ""}
                ><span>
                    {lang["settings:account.html:title.blocks"]}
                </span></a
            >
        {/if}
    </div>

    <!-- panel -->
    <div id="panel" style="display: contents">
        {@render children()}
    </div>
</div>
