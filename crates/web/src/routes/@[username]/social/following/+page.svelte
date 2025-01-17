<script lang="ts">
    import { Option } from "$lib/classes/Option.js";
    import ProfileCard from "$lib/components/ProfileCard.svelte";

    import { active_page } from "$lib/stores";
    active_page.set("social.following");

    const { data } = $props();
    const user = Option.from(data.user);
    const lang = data.lang;
    const page = data.data;
    const query = data.query;

    const { following, following_count } = page;
</script>

<div id="following" class="flex flex-col items-center gap-4">
    {#each following as card}
        <ProfileCard user={card[2]} {lang} current_profile={user} />
    {/each}

    <!-- pagination buttons -->
    {#if following_count !== 0}
        <div class="flex justify-between gap-2 w-full">
            {#if query.page > 0}
                <a class="button secondary" href="?page={query.page - 1}" data-sveltekit-reload
                    >{lang["general:link.previous"]}</a
                >
            {:else}
                <div></div>
            {/if}

            {#if following.length !== 0}
                <a class="button secondary" href="?page={query.page + 1}" data-sveltekit-reload
                    >{lang["general:link.next"]}</a
                >
            {/if}
        </div>
    {/if}
</div>
