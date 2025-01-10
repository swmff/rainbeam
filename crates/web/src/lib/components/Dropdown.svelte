<script lang="ts">
    import { onMount } from "svelte";
    const { children } = $props();

    const id = crypto.randomUUID();
    let open = $state(false);

    function clicked() {
        open = !open;
    }

    onMount(() => {
        document.documentElement.addEventListener("click", (e) => {
            if ((e.target as HTMLElement).id !== id) {
                return;
            }

            open = false;
        });
    });
</script>

<div
    class="dropdown"
    onclick={clicked}
    onkeydown={() => {}}
    role="button"
    tabindex="0"
    {id}
>
    {@render children()}
</div>
