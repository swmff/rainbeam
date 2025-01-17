<script lang="ts">
    import { active_page } from "$lib/stores.js";
    active_page.set("profile");

    import { Option } from "$lib/classes/Option";
    import GlobalQuestion from "$lib/components/GlobalQuestion.svelte";
    import Warning from "$lib/components/Warning.svelte";
    import Listing from "$lib/components/chats/Listing.svelte";
    import { Ellipsis, Plus } from "lucide-svelte";
    import { onMount } from "svelte";

    const { data } = $props();
    const lang = data.lang;
    const page_data = data.data;
    const user = Option.from(data.user);

    const { other, is_helper, is_powerful, response_count, questions_count, tokens_src, warnings, chats, badges } =
        page_data;
    const tokens = tokens_src as Array<string>;

    // functions
    onMount(() => {
        (globalThis as any).ban_ip = function (ip: string) {
            const reason = prompt("Please explain your reason for banning this IP below:");

            if (!reason) {
                return;
            }

            fetch("/api/v0/auth/ipbans", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    ip,
                    reason
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [res.success ? "success" : "error", res.success ? "IP banned!" : res.message]);
                });
        };

        (globalThis as any).change_group = async () => {
            const group = await trigger("app:prompt", ["Enter group number:"]);

            if (!group) {
                return;
            }

            if (group !== "-1" && group !== "0") {
                return alert("Cannot grant moderator permissions to other users.");
            }

            fetch(`/api/v0/auth/profile/${other.id}/group`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    group: Number.parseInt(group)
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [
                        res.success ? "success" : "error",
                        res.success ? "Group updated!" : res.message
                    ]);
                });
        };

        (globalThis as any).change_tier = async () => {
            const tier = await trigger("app:prompt", ["Enter tier number:"]);

            if (!tier) {
                return;
            }

            fetch(`/api/v0/auth/profile/${other.id}/tier`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    tier: Number.parseInt(tier)
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [
                        res.success ? "success" : "error",
                        res.success ? "Tier updated!" : res.message
                    ]);
                });
        };

        (globalThis as any).change_coins = async () => {
            const coins = await trigger("app:prompt", ["Enter coin amount:"]);

            if (!coins) {
                return;
            }

            fetch(`/api/v0/auth/profile/${other.id}/coins`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    coins: Number.parseInt(coins)
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [
                        res.success ? "success" : "error",
                        res.success ? "Coins updated!" : res.message
                    ]);
                });
        };

        (globalThis as any).patch_metadata = async (metadata: any) => {
            fetch(`/api/v0/auth/profile/${other.id}/metadata`, {
                method: "PUT",
                headers: {
                    "Content-Type": "application/json"
                },
                body: JSON.stringify({
                    metadata
                })
            })
                .then((res) => res.json())
                .then((res) => {
                    trigger("app:toast", [
                        res.success ? "success" : "error",
                        res.success ? "Metadata updated!" : res.message
                    ]);
                });
        };

        (globalThis as any).change_verify = async () => {
            const verify_url = await trigger("app:prompt", ["Enter verify URL:"]);

            if (!verify_url) {
                return;
            }

            const verify_code = window.crypto.randomUUID();

            await (globalThis as any).patch_metadata({
                kv: {
                    "rainbeam:verify_url": verify_url,
                    "rainbeam:verify_code": verify_code
                }
            });
        };

        const tokens = other.tokens;
        (globalThis as any).remove_session = async (id: string) => {
            if (!(await trigger("app:confirm", ["Are you sure you want to do this?"]))) {
                return;
            }

            tokens.splice(tokens.indexOf(id), 1);
            (document.getElementById(`session:${id}`) as HTMLElement).remove();
            (globalThis as any).save_sessions();
        };

        (globalThis as any).save_sessions = async () => {
            const res = await (
                await fetch(`/api/v0/auth/profile/${other.id}/tokens`, {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json"
                    },
                    body: JSON.stringify({
                        tokens
                    })
                })
            ).json();

            trigger("app:toast", [res.success ? "success" : "error", res.success ? "Sessions saved!" : res.message]);
        };

        (globalThis as any).create_token = async () => {
            const app_name = await trigger("app:prompt", ["App identifier:"]);
            if (!app_name) {
                return;
            }

            const permissions = await trigger("app:prompt", ["Permissions (comma separated):"]);

            const res = await (
                await fetch(`/api/v0/auth/profile/${other.id}/tokens/generate`, {
                    method: "POST",
                    headers: {
                        "Content-Type": "application/json"
                    },
                    body: JSON.stringify({
                        app: app_name,
                        permissions: permissions ? permissions.split(",") : []
                    })
                })
            ).json();

            trigger("app:toast", [res.success ? "success" : "error", res.success ? "Token generated!" : res.message]);

            if (res.success) {
                alert(res.payload);
            }
        };
    });
</script>

<div class="pillmenu convertible shadow">
    <a href="/@{other.username}">
        <span
            >{lang["profile:link.feed"]}
            <b class="notification">{response_count}</b></span
        >
    </a>

    <a href="/@{other.username}/questions">
        <span>Questions <b class="notification">{questions_count}</b></span>
    </a>

    {#if is_helper}
        <a href="/@{other.username}/mod" class="active">
            <span>{lang["profile:link.manage"]} <b class="notification">Mod</b></span>
        </a>
    {/if}
</div>

<div class="pillmenu convertible shadow true">
    <a href="#/info" class="active" data-tab-button="info"><span>Info</span></a>

    {#if is_powerful}
        <a href="#/badges" data-tab-button="badges"><span>Badges</span></a>
        <a href="#/password" data-tab-button="password"><span>Password</span></a>
    {/if}

    <a href="#/sessions" data-tab-button="sessions"><span>Sessions</span></a>
    <a href="#/chats" data-tab-button="chats"><span>Chats</span></a>
    <a href="#/warnings" data-tab-button="warnings"><span>Warnings</span></a>
</div>

<!-- info -->
<div data-tab="info">
    <div id="info" class="flex flex-col gap-4">
        <div class="card w-full shadow">
            <ul style="margin-bottom: 0">
                <li>Joined: <span class="date">{other.joined}</span></li>
                <!-- svelte-ignore a11y_invalid_attribute -->
                <li>Group: <a href="javascript:change_group()">{other.group}</a></li>
                <!-- svelte-ignore a11y_invalid_attribute -->
                <li>Tier: <a href="javascript:change_tier()">{other.tier}</a></li>
                <!-- svelte-ignore a11y_invalid_attribute -->
                <li>Coins: <a href="javascript:change_coins()">{other.coins}</a></li>

                {#if other.metadata.kv["rainbeam:verify_url"] && other.metadata.kv["rainbeam:verify_code"]}
                    <li>
                        Verify URL: <a href={other.metadata.kv["rainbeam:verify_url"]}
                            >{other.metadata.kv["rainbeam:verify_url"]}</a
                        >
                    </li>
                    <li>
                        <!-- svelte-ignore a11y_invalid_attribute -->
                        Verify code:
                        <a href="javascript:change_verify()">{other.metadata.kv["rainbeam:verify_code"]}</a>
                    </li>
                {:else}
                    <!-- svelte-ignore a11y_invalid_attribute -->
                    <li><a href="javascript:change_verify()">Set verify</a></li>
                {/if}

                <li><a href="/inbox/mail?profile={other.id}">View mail</a></li>
                <li><a href="/market?creator={other.id}">View items</a></li>
                <li><a href="/settings?profile={other.id}">View settings</a></li>
                <li><a href="/inbox/notifications?profile={other.id}">View notifications</a></li>
            </ul>

            <div class="flex gap-2 flex-wrap">
                {#each Object.entries(other.metadata.kv) as kv}
                    <details>
                        <summary class="flex items-center gap-2">
                            <Ellipsis class="icon" />
                            <code style="background: transparent">{kv[0]}</code>
                        </summary>
                        <pre><code>{kv[1]}</code></pre>
                    </details>
                {/each}
            </div>
        </div>
    </div>
</div>

{#if is_powerful}
    <!-- badges -->
    <div data-tab="badges" class="hidden">
        <div id="badges" class="flex flex-col gap-4">
            <div class="card w-full shadow">
                <form
                    class="flex flex-col gap-1"
                    id="badges_form"
                    onsubmit={async (e) => {
                        e.preventDefault();

                        if (!(await trigger("app:confirm", ["Are you sure you want to do this?"]))) {
                            return;
                        }

                        fetch(`/api/v0/auth/profile/${other.id}/badges`, {
                            method: "POST",
                            headers: {
                                "Content-Type": "application/json"
                            },
                            body: JSON.stringify({
                                badges: JSON.parse((e.target as any).badges.value)
                            })
                        })
                            .then((res) => res.json())
                            .then((res) => {
                                trigger("app:toast", [
                                    res.success ? "success" : "error",
                                    res.message || "Badges updated!"
                                ]);
                            });
                    }}
                >
                    <label for="badges_data">Badges data</label>

                    <textarea name="badges" id="badges_data" required>{badges}</textarea>

                    <button>{lang["general:form.submit"]}</button>
                </form>
            </div>
        </div>
    </div>

    <!-- change password -->
    <div data-tab="password" class="hidden">
        <div class="flex flex-col gap-4">
            <h3>Change password</h3>

            <form
                class="card shadow flex flex-col gap-1"
                id="change_password"
                onsubmit={async (e) => {
                    e.preventDefault();

                    if (!(await trigger("app:confirm", ["Are you sure you want to do this?"]))) {
                        return;
                    }

                    fetch(`/api/v0/auth/profile/${other.id}/password`, {
                        method: "POST",
                        headers: {
                            "Content-Type": "application/json"
                        },
                        body: JSON.stringify({
                            password: "",
                            new_password: (e.target as any).new_password.value
                        })
                    })
                        .then((res) => res.json())
                        .then((res) => {
                            trigger("app:shout", [res.success ? "tip" : "caution", res.message || "Password changed!"]);

                            window.location.href = "#top";
                            (e.target as any).reset();
                        });
                }}
            >
                <label for="new_password">New password</label>
                <input type="password" name="new_password" id="new_password" minlength="6" />

                <button>{lang["general:form.submit"]}</button>
            </form>
        </div>
    </div>
{/if}

<!-- sessions -->
<div data-tab="sessions" class="hidden">
    <div id="sessions" class="flex flex-col gap-4">
        <div class="flex w-full gap-2 justify-between items-center">
            <div></div>
            <!-- svelte-ignore a11y_invalid_attribute -->
            <a href="javascript:create_token()" class="button primary bold">
                <Plus class="icon" /> New
            </a>
        </div>

        <div class="card w-full shadow" style="overflow: auto">
            <table class="w-full">
                <thead>
                    <tr>
                        <th>IP</th>
                        <th>App</th>
                        <th>Permissions</th>
                        <th>Actions</th>
                    </tr>
                </thead>

                <tbody>
                    {#each Object.entries(tokens) as [i, session]}
                        <tr id="session:{session}" title={session.substring(0, 10)}>
                            {#if other.ips[i]}
                                {@const ip = other.ips[i]}
                                <td style="white-space: nowrap">
                                    {#if !ip}
                                        <span class="tag">None</span>
                                    {:else}
                                        <a href="javascript:globalThis.ban_ip('{ip}')">{ip}</a>
                                    {/if}
                                </td>
                            {:else}
                                <td></td>
                            {/if}

                            {#if other.token_context[i]}
                                {@const ctx = other.token_context[i]}
                                <td style="white-space: nowrap">
                                    {#if !ctx.app}
                                        <span class="tag">None</span>
                                    {:else}
                                        {ctx.app}
                                    {/if}
                                </td>
                            {:else}
                                <td style="white-space: nowrap">
                                    <span class="tag">None</span>
                                </td>
                            {/if}

                            {#if other.token_context[i]}
                                {@const ctx = other.token_context[i]}
                                {#if ctx.permissions}
                                    {@const permissions = ctx.permissions}
                                    <td style="white-space: nowrap">
                                        {#if permissions === ""}
                                            <span class="tag">None</span>
                                        {:else}
                                            <ul>
                                                {#each permissions as permission}
                                                    <li>
                                                        {permission}
                                                    </li>
                                                {/each}
                                            </ul>
                                        {/if}
                                    </td>
                                {:else}
                                    <td style="white-space: nowrap">
                                        <span class="tag">All</span>
                                    </td>
                                {/if}
                            {:else}
                                <td></td>
                            {/if}

                            <td>
                                <a href="javascript:remove_session('{session}')">Delete</a>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    </div>
</div>

<!-- chats -->
<div data-tab="chats" class="hidden">
    <div id="chats" class="card shadow flex flex-col gap-4">
        {#each chats as chat}
            <Listing {chat} />
        {/each}
    </div>
</div>

<!-- warnings -->
<div data-tab="warnings" class="hidden">
    <div class="flex flex-col gap-4">
        <div class="card-nest shadow w-full" id="warning_field">
            <div class="card shadow flex flex-col gap-1">Create a warning</div>

            <div class="card shadow">
                <form
                    class="flex flex-col gap-2"
                    onsubmit={(e) => {
                        e.preventDefault();
                        fetch("/api/v0/auth/warnings", {
                            method: "POST",
                            headers: {
                                "Content-Type": "application/json"
                            },
                            body: JSON.stringify({
                                recipient: other.id,
                                content: (e.target as any).content.value
                            })
                        })
                            .then((res) => res.json())
                            .then((res) => {
                                trigger("app:toast", [
                                    res.success ? "success" : "error",
                                    res.success ? "User warned!" : res.message
                                ]);

                                if (res.success === true) {
                                    (e.target as any).reset();
                                }
                            });
                    }}
                >
                    <textarea
                        class="w-full"
                        placeholder="Type your warning!"
                        minlength="1"
                        required
                        name="content"
                        id="content"
                    ></textarea>

                    <div class="flex justify-between w-full gap-1">
                        <div></div>
                        <button class="primary bold">
                            {lang["general:form.submit"]}
                        </button>
                    </div>
                </form>
            </div>
        </div>

        {#each warnings as warning}
            <Warning {warning} profile={user} />
        {/each}
    </div>
</div>
