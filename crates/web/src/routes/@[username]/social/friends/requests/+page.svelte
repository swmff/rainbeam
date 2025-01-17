<script lang="ts">
    import { Option } from "$lib/classes/Option.js";

    import { active_page } from "$lib/stores";
    active_page.set("social.requests");

    const { data } = $props();
    const user = Option.from(data.user).unwrap();
    const lang = data.lang;
    const page = data.data;

    const { requests, other } = page;
</script>

<div id="requests" class="flex flex-col items-center gap-4">
    <table class="w-full">
        <thead>
            <tr>
                <th>Type</th>
                <th>User</th>
                <th>Actions</th>
            </tr>
        </thead>

        <tbody>
            {#each requests as request}
                {@const outbound = request[0].id === other.id}
                <tr>
                    <td
                        >{#if outbound}Outbound{:else}Inbound{/if}</td
                    >

                    <td>
                        {#if outbound}
                            <a href="/@{request[1].username}">
                                {request[1].username}
                            </a>
                        {:else}
                            <a href="/@{request[0].username}">
                                {request[0].username}
                            </a>
                        {/if}
                    </td>

                    <td>
                        {#if !outbound}
                            <a href="/@{request[0].username}/relationship/friend_accept">Accept</a>
                        {:else}
                            <a href="javascript:cancel_fr('{request[1].username}')">Cancel</a>
                        {/if}
                    </td>
                </tr>
            {/each}
        </tbody>
    </table>
</div>
