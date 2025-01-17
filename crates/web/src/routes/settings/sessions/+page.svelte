<script lang="ts">
    import { active_page } from "$lib/stores";
    active_page.set("settings.sessions");

    import { Option } from "$lib/classes/Option";
    import { onMount } from "svelte";

    const { data } = $props();
    const user = Option.from(data.user).unwrap();
    const lang = data.lang;
    const page = data.data;

    const { tokens_src, current_session } = page;
    const tokens = tokens_src as Array<string>;

    onMount(() => {
        const tokens = user.tokens;
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
                await fetch("/api/v0/auth/me/tokens", {
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
    });
</script>

<div class="flex flex-col gap-4">
    <div class="flex flex-col gap-1" id="manage_sessions" style="overflow: auto">
        <h4 class="title">
            {lang["settings:sessions.html:title.sessions"]}
        </h4>

        <table>
            <thead>
                <tr>
                    <th>{lang["settings:sessions.html:label.tag"]}</th>
                    <th>IP</th>
                    <th>{lang["settings:sessions.html:label.app"]}</th>
                    <th>
                        {lang["settings:sessions.html:label.permissions"]}
                    </th>
                    <th>{lang["settings:sessions.html:label.actions"]}</th>
                </tr>
            </thead>

            <tbody>
                {#each Object.entries(tokens) as [i, session]}
                    <tr id="session:{session}" title="{session.substring(0, 10)} }">
                        <td style="white-space: nowrap">
                            {#if current_session === session}
                                <span class="notification marker">{lang["settings:sessions.html:text.active"]}</span>
                            {:else}
                                <span class="tag">{lang["settings:sessions.html:text.none"]}</span>
                            {/if}
                        </td>

                        {#if user.ips[i]}
                            {@const ip = user.ips[i]}
                            <td style="white-space: nowrap">
                                {#if !ip}
                                    <span class="tag">{lang["settings:sessions.html:text.none"]}</span>
                                {:else}
                                    {{ ip }}
                                {/if}
                            </td>
                        {:else}
                            <td></td>
                        {/if}

                        {#if user.token_context[i]}
                            {@const ctx = user.token_context[i]}
                            <td style="white-space: nowrap">
                                {#if !ctx.app}
                                    <span class="tag">None</span>
                                {:else}
                                    {ctx.app}
                                {/if}
                            </td>
                        {:else}
                            <td style="white-space: nowrap">
                                <span class="tag">{lang["settings:sessions.html:text.none"]}</span>
                            </td>
                        {/if}

                        {#if user.token_context[i]}
                            {@const ctx = user.token_context[i]}
                            {#if ctx.permissions}
                                {@const permissions = ctx.permissions}
                                <td style="white-space: nowrap">
                                    {#if !permissions}
                                        <span class="tag">{lang["settings:sessions.html:text.none"]}</span>
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
                                    <span class="tag">{lang["settings:sessions.html:text.all"]}</span>
                                </td>
                            {/if}
                        {:else}
                            <td></td>
                        {/if}

                        <td>
                            <a href="javascript:remove_session('{session}')">{lang["general:action.delete"]}</a>
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
</div>
