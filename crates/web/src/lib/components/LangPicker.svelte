<script lang="ts">
    import type { LangFile } from "$lib/bindings/LangFile";
    import { onMount } from "svelte";

    const { lang }: { lang: LangFile["name"] } = $props();

    onMount(() => {
        (
            document.querySelector(`option[value="${lang}"]`) as HTMLElement
        ).setAttribute("selected", "true");
    });
</script>

<select
    name="language"
    id="language"
    onchange={(event) => {
        const lang = (event.target as any).options[
            (event.target as any).selectedIndex
        ].value;

        fetch(`/api/v0/util/lang/set?id=${lang}`, { method: "POST" }).then(
            () => {
                window.location.reload();
            }
        );
    }}
>
    <option value="net.rainbeam.langs:en-US">🇺🇸 en-US</option>
    <option value="net.rainbeam.langs:ko-KR">🇰🇷 ko-KR</option>
</select>
