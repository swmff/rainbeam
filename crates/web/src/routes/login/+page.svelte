<script lang="ts">
    import { onMount } from "svelte";

    const { data } = $props();
    const config = data.config;
    const lang = data.lang;

    onMount(() => {
        const error = document.getElementById("error") as HTMLElement;
        const success = document.getElementById("success") as HTMLElement;
        const forms = document.getElementById("forms") as HTMLElement;
        const callback = "/api/v0/auth/callback";

        (document.getElementById("login_form") as any).addEventListener("submit", async (e: any) => {
            e.preventDefault();
            const res = await fetch("/api/v0/auth/login", {
                method: "POST",
                body: JSON.stringify({
                    username: e.target.username.value,
                    password: e.target.password.value,
                    token: e.target.querySelector(".h-captcha textarea").value
                }),
                headers: {
                    "Content-Type": "application/json"
                }
            });

            const json = await res.json();

            if (json.success === false) {
                error.style.display = "block";
                error.innerHTML = `<div class="mdnote-title">${json.message}</div>`;
            } else {
                success.style.display = "flex";
                success.innerHTML = `<p>Successfully logged into account.</p>

                <hr />
                <a href="${callback}?uid=${json.message}" class="button login bold">Continue</a>`;
                forms.style.display = "none";
            }
        });
    });
</script>

<article class="flex flex-col gap-1 items-center page_section">
    <main class="flex flex-col gap-2">
        <div id="success" class="card flex flex-col gap-2" style="display: none; width: 100%"></div>
        <div id="error" class="markdown-alert-caution" style="display: none; width: 100%"></div>

        <div id="forms" class="flex flex-col gap-2 items-center">
            <h3 class="no-margin">{lang["auth:login.html:title.login"]}</h3>

            <div class="card" style="width: 25rem">
                <form id="login_form" class="flex flex-col gap-2">
                    <div class="row flex flex-col gap-1">
                        <label for="username">{lang["auth:label.username"]}</label>
                        <input type="text" name="username" id="username" required minlength="2" maxlength="32" />
                    </div>

                    <div class="row flex flex-col gap-1">
                        <label for="password">{lang["auth:label.password"]}</label>
                        <input type="password" name="password" id="password" required minlength="6" />
                    </div>

                    <div class="h-captcha" data-sitekey={config.captcha.site_key}></div>

                    <hr />

                    <button class="primary bold">
                        {lang["general:link.login"]}
                    </button>
                </form>
            </div>

            <p>
                {lang["auth:login.html:text.no_account"]}
                <a href="/sign_up" data-turbo="false" data-sveltekit-reload>{lang["general:link.sign_up"]}</a>
            </p>
        </div>
    </main>
</article>
