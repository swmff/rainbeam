<script lang="ts">
    import type { LangFile } from "$lib/bindings/LangFile";
    import type { Profile } from "$lib/bindings/Profile";
    import type { Option } from "$lib/classes/Option";
    import { Ellipsis } from "lucide-svelte";
    import { onMount } from "svelte";

    const {
        lang,
        profile
    }: { lang: LangFile["data"]; profile: Option<Profile> } = $props();

    onMount(() => {
        const user = profile.unwrap();

        if (
            user.metadata.kv["sparkler:private_profile"] ||
            user.metadata.kv["rainbeam:nsfw_profile"]
        ) {
            (document.getElementById("unlisted") as any).checked = true;
        }
    });
</script>

<details>
    <summary class="icon-only"><Ellipsis class="icon" /></summary>

    <div class="flex flex-col gap-2 card round shadow-md">
        <input
            type="text"
            name="tags"
            id="tags"
            placeholder="Tags (optional)"
        />

        <p class="fade">Tags should be separated by a comma.</p>

        <input
            type="text"
            name="warning"
            id="warning"
            placeholder="Warning (optional), leave blank for no warning"
        />

        <p class="fade">
            Users must accept this warning to view your post's contents. This
            may be required if your post contains sensitive content.
        </p>

        <input
            type="text"
            name="reply"
            id="reply"
            placeholder="Reply to ID (optional), leave blank for no reply"
        />

        <p class="fade">
            Putting the full ID of another response here will mark your response
            as a reply to the response you mentioned here.
        </p>

        <input
            type="text"
            name="circle"
            id="circle"
            placeholder="Post in circle (optional), leave blank for profile"
        />

        <p class="fade">
            Putting the full ID of a circle here will publish this post to the
            specified circle.
        </p>

        <div class="checkbox_container">
            <input type="checkbox" name="unlisted" id="unlisted" />
            <label for="unlisted" class="normal">Unlisted</label>
        </div>

        <p class="fade">
            Unlisted responses will be hidden from <b>public</b> timelines.
        </p>
    </div>
</details>
