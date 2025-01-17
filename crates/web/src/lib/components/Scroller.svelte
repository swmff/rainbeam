<script lang="ts">
    import { onMount, onDestroy } from "svelte";

    const { load, threshold = 0 }: { load: any; threshold: number } = $props();

    let load_more = false;
    let has_more = true;
    let component: HTMLElement | null = null;

    onMount(() => {
        const scroll_element = document.body;
        if (component || scroll_element) {
            const element = scroll_element
                ? scroll_element
                : component?.parentNode;

            element?.addEventListener("scroll", on_scroll);
            element?.addEventListener("resize", on_scroll);
        }

        return () => {
            window.removeEventListener("scroll", on_scroll);
        };
    });

    const on_scroll = (e: any) => {
        const offset =
            e.target.scrollHeight - e.target.clientHeight - e.target.scrollTop;

        if (offset <= threshold) {
            if (!load_more && has_more) {
                load();
            }

            load_more = true;
        } else {
            load_more = false;
        }
    };
</script>

<div bind:this={component} style="width: 0px"></div>
