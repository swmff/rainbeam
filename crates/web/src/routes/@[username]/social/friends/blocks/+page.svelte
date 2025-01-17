<script lang="ts">
    import { Option } from "$lib/classes/Option.js";

    import { active_page } from "$lib/stores";
    active_page.set("social.blocks");

    const { data } = $props();
    const lang = data.lang;
    const page = data.data;
    const query = data.query;

    const { blocks, other } = page;
</script>

<div id="requests" class="flex flex-col items-center gap-4">
    <table class="w-full">
        <thead>
            <tr>
                <th>Type</th>
                <th>User</th>
            </tr>
        </thead>

        <tbody>
            {#each blocks as block}
                {@const outbound = block[0].id === other.id}
                <tr>
                    <td
                        >{#if outbound}Outbound{:else}Inbound{/if}</td
                    >

                    <td>
                        {#if outbound}
                            <a href="/@{block[1].username}">
                                {block[1].username}
                            </a>
                        {:else}
                            <a href="/@{block[0].username}">
                                {block[0].username}
                            </a>
                        {/if}
                    </td>
                </tr>
            {/each}
        </tbody>
    </table>

    <!-- pagination buttons -->
    {#if blocks.length !== 0}
        <div class="flex justify-between gap-2 w-full">
            {#if query.page > 0}
                <a class="button secondary" href="?page={query.page - 1}" data-sveltekit-reload
                    >{lang["general:link.previous"]}</a
                >
            {:else}
                <div></div>
            {/if}

            <a class="button secondary" href="?page={query.page + 1}" data-sveltekit-reload
                >{lang["general:link.next"]}</a
            >
        </div>
    {/if}
</div>
