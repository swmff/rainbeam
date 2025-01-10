<svelte:head>
    <link rel="stylesheet" href="/css/style.css" />

    <script lang="ts">
        function media_theme_pref() {
            document.documentElement.removeAttribute("class");

            if (
                window.matchMedia("(prefers-color-scheme: dark)").matches &&
                !window.localStorage.getItem("theme")
            ) {
                document.documentElement.classList.add("dark");
            } else if (
                window.matchMedia("(prefers-color-scheme: light)").matches &&
                !window.localStorage.getItem("theme")
            ) {
                document.documentElement.classList.remove("dark");
            } else if (window.localStorage.getItem("theme")) {
                const current = window.localStorage.getItem("theme");
                document.documentElement.className = current;
            }
        }

        media_theme_pref();
        document.documentElement.addEventListener("load", () => {
            if (!document.getElementById("theme")) {
                return;
            }

            const profile_theme = document
                .getElementById("theme")
                .innerText.trim();

            if (profile_theme) {
                return;
            }

            media_theme_pref();
        });
    </script>
</svelte:head>

<div id="page">
    <div class="sidebar">
        <nav>
            <a href="/" class="button desktop title">
                <img
                    src="/images/ui/logo.svg"
                    alt="name"
                    width="32px"
                    height="32px"
                    class="title-content"
                    id="title-img"
                />
            </a>

            <b class="title-content" style="display: none">name</b>
        </nav>
    </div>

    <div class="content_container" id="page_content">
        <article>
            <main class="flex flex-col gap-2">
                <slot />
            </main>
        </article>
    </div>

    <div class="sidebar"></div>
</div>
