<script lang="ts">
    import ClickOutside from "$lib/events/ClickOutside";

    const { children, classname = "" } = $props();
    let open = $state(false);
</script>

<div
    class="dropdown {classname} {open ? 'open' : ''}"
    onclick={(event: any) => {
        if ((globalThis as any).__close_dropdown) {
            (globalThis as any).__close_dropdown();
        }

        (globalThis as any).__close_dropdown = () => {
            open = false;
        };

        open = true;
        event.stopPropagation();

        let target = event.target;
        while (!target.classList.contains("dropdown")) {
            target = target.parentElement!;
        }

        const dropdown = target.children[1];

        // check y
        const box = target.getBoundingClientRect();
        let parent = target.parentElement;

        while (!parent.matches("html, .window")) {
            parent = parent.parentElement;
        }

        let parent_height = parent.getBoundingClientRect().y;

        if (parent.nodeName === "HTML") {
            parent_height = window.screen.height;
        }

        const scroll = window.scrollY;
        const height = parent_height;
        const y = box.y + scroll;

        if (y > height - scroll - 300) {
            dropdown.classList.add("top");
        } else {
            dropdown.classList.remove("top");
        }
    }}
    use:ClickOutside={() => {
        open = false;
        (globalThis as any).__close_dropdown = undefined;
    }}
    onkeydown={() => {}}
    tabindex="0"
    role="button"
>
    {@render children()}
</div>
