<script lang="ts">
    import ClickOutside from "$lib/events/ClickOutside";

    const { children, classname } = $props();
    let open = $state(false);
</script>

<button
    class="dropdown {classname} {open ? 'open' : ''}"
    onclick={(event: any) => {
        open = true;
        event.stopPropagation();
    }}
    use:ClickOutside={() => {
        open = false;
    }}
>
    {@render children()}
</button>
