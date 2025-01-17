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

        (document.getElementById("register_form") as any).addEventListener("submit", async (e: any) => {
            e.preventDefault();

            // sign up
            const res = await fetch("/api/v0/auth/register", {
                method: "POST",
                body: JSON.stringify({
                    username: e.target.username.value,
                    password: e.target.password.value,
                    token: e.target.querySelector(".h-captcha textarea").value,
                    policy_consent: e.target.policy_consent.checked
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
                success.innerHTML = `<p>Account successfully created, welcome!</p>

                <hr />
                <a href="${callback}?uid=${json.message}" class="button primary bold">Continue</a>`;
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
            <h3 class="no-margin">
                {lang["auth:sign_up.html:title.sign_up"]}
            </h3>

            <div class="card" style="width: 25rem">
                <form id="register_form" class="flex flex-col gap-2">
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

                    <p>
                        By continuing, you agree to our
                        <a href="/site/terms-of-service">terms of service</a>
                        and <a href="/site/privacy">privacy policy</a>.
                    </p>

                    <div class="checkbox_container">
                        <input type="checkbox" required id="policy_consent" name="policy_consent" />

                        <label for="policy_consent" class="normal">I agree</label>
                    </div>

                    <hr />

                    <button class="primary bold">
                        {lang["general:link.sign_up"]}
                    </button>
                </form>
            </div>

            <p>
                {lang["auth:sign_up.html:text.has_account"]}
                <a href="/login" data-turbo="false" data-sveltekit-reload>{lang["general:link.login"]}</a>
            </p>
        </div>
    </main>
</article>
