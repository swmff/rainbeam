<script lang="ts">
    import Response from "$lib/components/Response.svelte";
    import type { Profile } from "$lib/bindings/Profile.js";
    import { Option } from "$lib/classes/Option";

    const { data } = $props();
    const page = data.data;

    const profile = Option.from(data.user) as Option<Profile>;
    const config = data.config;
    const lang = data.lang;
    const do_render_nested = data.query.do_render_nested === "true";

    const {
        response,
        anonymous_avatar,
        anonymous_username,
        is_pinned,
        is_powerful,
        is_helper,
        show_pin_button
    } = page;
</script>

<Response
    res={response}
    {anonymous_avatar}
    {anonymous_username}
    {is_pinned}
    {is_powerful}
    {is_helper}
    {show_pin_button}
    {profile}
    {config}
    {lang}
    {do_render_nested}
/>
