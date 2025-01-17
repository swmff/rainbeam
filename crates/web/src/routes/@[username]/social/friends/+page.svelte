<script lang="ts">
    import { Option } from "$lib/classes/Option.js";
    import ProfileCard from "$lib/components/ProfileCard.svelte";

    import { active_page } from "$lib/stores";
    active_page.set("social.friends");

    const { data } = $props();
    const user = Option.from(data.user);
    const lang = data.lang;
    const page = data.data;
    const query = data.query;

    const { friends, friends_count, other } = page;
</script>

<div id="friends" class="flex flex-col items-center gap-4">
    {#each friends as relationship}
        {#if other.id !== relationship[0].id}
            <ProfileCard user={relationship[0]} {lang} current_profile={user} />
        {:else}
            <ProfileCard user={relationship[1]} {lang} current_profile={user} />
        {/if}
    {/each}

    <!-- pagination buttons -->
    {#if friends_count !== 0}
        <div class="flex justify-between gap-2 w-full">
            {#if query.page > 0}
                <a class="button secondary" href="?page={query.page - 1}" data-sveltekit-reload
                    >{lang["general:link.previous"]}</a
                >
            {:else}
                <div></div>
            {/if}

            {#if friends.length !== 0}
                <a class="button secondary" href="?page={query.page + 1}" data-sveltekit-reload
                    >{lang["general:link.next"]}</a
                >
            {/if}
        </div>
    {/if}
</div>
