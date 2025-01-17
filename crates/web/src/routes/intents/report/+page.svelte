<script lang="ts">
    import { onMount } from "svelte";

    const { data } = $props();
    const lang = data.lang;
    const config = data.config;
    const query = data.query;

    onMount(() => {
        setTimeout(() => {
            trigger("reports:fill", [query.type, query.target]);
        }, 100);
    });
</script>

<article>
    <main class="flex flex-col gap-2">
        <form
            class="card shadow-md"
            onsubmit={(event) => {
                trigger("reports:file", [event]);
            }}
        >
            <div class="flex flex-col gap-1">
                <label for="content">{lang["report.html:label.reason"]}</label>

                <textarea name="content" id="content" required minlength="5"
                ></textarea>

                <p class="fade">
                    {lang["report.html:text.please_describe"]}
                </p>

                <p class="fade">{lang["report.html:text.details1"]}</p>
                <p class="fade">{lang["report.html:text.details2"]}</p>
            </div>

            <div class="h-captcha" data-sitekey={config.captcha.site_key}></div>

            <hr />
            <div class="flex gap-2">
                <button class="primary bold">
                    {lang["general:action.report"]}
                </button>

                <button
                    class="bold"
                    type="button"
                    onclick={() => {
                        window.close();
                    }}
                >
                    {lang["general:dialog.cancel"]}
                </button>
            </div>
        </form>
    </main>
</article>
