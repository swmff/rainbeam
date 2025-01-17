<script lang="ts">
    import { active_page } from "$lib/stores";
    import { Option } from "$lib/classes/Option";
    import { BadgePlus, ChevronRight, CircleUserRound, ExternalLink, Store, Wallet } from "lucide-svelte";
    active_page.set("settings.coins");

    const { data } = $props();
    const lang = data.lang;
    const page = data.data;
    const user = Option.from(data.user).unwrap();

    const { transactions } = page;
</script>

<div class="flex flex-col gap-4">
    <div class="pillmenu convertible shadow true">
        <a href="#/balance" class="active" data-tab-button="balance"
            ><span>{lang["settings:coins.html:title.balance"]}</span></a
        >

        <a href="#/transactions" data-tab-button="transactions"
            ><span>{lang["settings:coins.html:title.transactions"]}</span></a
        >
    </div>

    <!-- balance -->
    <div data-tab="balance" class="flex flex-col gap-4">
        <div class="w-full flex justify-between gap-2">
            <div></div>

            <a href="#/transactions" class="button bold">
                <Wallet class="icon" /> <span>{user.coins}</span>
            </a>
        </div>

        <div class="card secondary flex flex-col gap-2">
            <a href="/market" class="card w-full flex justify-between items-center gap-2">
                <div class="flex gap-2 items-center">
                    <Store class="icon" />
                    <span>
                        {lang["settings:coins.html:text.browse_market"]}
                    </span>
                </div>

                <button class="primary icon-only small">
                    <ChevronRight class="icon" />
                </button>
            </a>

            <a href="/market/new" class="card w-full flex justify-between items-center gap-2">
                <div class="flex gap-2 items-center">
                    <BadgePlus class="icon" />

                    <span>
                        {lang["settings:coins.html:text.publish_item"]}
                    </span>
                </div>

                <button class="primary icon-only small">
                    <ChevronRight class="icon" />
                </button>
            </a>

            <a href="/market?creator={user.id}" class="card w-full flex justify-between items-center gap-2">
                <div class="flex gap-2 items-center">
                    <CircleUserRound class="icon" />

                    <span>
                        {lang["settings:coins.html:text.my_items"]}
                    </span>
                </div>

                <button class="primary icon-only small">
                    <ChevronRight class="icon" />
                </button>
            </a>
        </div>
    </div>

    <!-- transactions -->
    <div class="flex flex-col gap-1 hidden" style="overflow: auto" data-tab="transactions">
        <table>
            <thead>
                <tr>
                    <th>{lang["settings:coins.html:label.amount"]}</th>
                    <th>{lang["settings:coins.html:label.customer"]}</th>
                    <th>{lang["settings:coins.html:label.merchant"]}</th>
                    <th>{lang["settings:coins.html:label.type"]}</th>
                    <th>{lang["settings:coins.html:label.item"]}</th>
                </tr>
            </thead>

            <tbody>
                {#each transactions as [[transaction, item], customer, merchant]}
                    <tr id="transaction:{transaction.id}">
                        <td>{transaction.amount}</td>
                        <td><a href="/@{customer.username}">{customer.username}</a></td>
                        <td><a href="/@{merchant.username}">{merchant.username}</a></td>

                        {#if item}
                            <td>{item.type}</td>
                            <td
                                ><a href="/market/item/{item.id}" title={item.name} class="flex items-center w-content"
                                    ><ExternalLink class="icon" /></a
                                ></td
                            >
                        {/if}
                    </tr>
                {/each}
            </tbody>
        </table>

        <!-- pagination buttons -->
        <div class="flex justify-between gap-2 w-full">
            {#if page > 0}
                <a class="button" href="?page={page - 1}#/transactions">{lang["general:link.previous"]}</a>
            {:else}
                <div></div>
            {/if}
            {#if transactions.length !== 0}
                <a class="button" href="?page={page + 1}#/transactions">{lang["general:link.next"]}</a>
            {/if}
        </div>
    </div>
</div>
