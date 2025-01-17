<script lang="ts">
    import { page } from "$app/state";

    const { data } = $props();
    const lang = data.lang;
</script>

<svelte:head>
    <title>{lang["error.html:link.uh_oh"]}</title>
</svelte:head>

<article>
    <main>
        <div class="card flex flex-col gap-2">
            <h4>{lang["error.html:link.uh_oh"]}</h4>
            <span>{page.error?.message} ({page.status})</span>

            <hr />
            <div class="flex gap-2">
                <a href="/" class="button">{lang["error.html:link.go_home"]}</a>
            </div>
        </div>
    </main>
</article>
