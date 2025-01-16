<script lang="ts">
    import {
        Award,
        Bomb,
        ChevronDown,
        Code,
        Copy,
        Crown,
        Ellipsis,
        Flag,
        LockKeyhole,
        MailPlus,
        MessageCirclePlus,
        Pen,
        Search,
        Settings,
        Shield,
        ShieldBan,
        Trash,
        Lock
    } from "lucide-svelte";
    import { onMount } from "svelte";

    import { active_page } from "$lib/stores.js";
    active_page.set("profile");

    import Question from "$lib/components/Question.svelte";
    import Dropdown from "$lib/components/Dropdown.svelte";
    import MoreResponseOptions from "$lib/components/MoreResponseOptions.svelte";
    import { Option } from "$lib/classes/Option";
    import Notification from "$lib/components/Notification.svelte";
    import type { RelationshipStatus } from "$lib/bindings/RelationshipStatus";
    import { render_markdown } from "$lib/helpers.js";
    import UserNote from "$lib/components/UserNote.svelte";

    const { data, children } = $props();
    const lang = data.lang;
    const page_data = data.data;
    const user = Option.from(data.user);
    const profile = user.unwrap();
    const config = data.config;
    const search_query = data.query;

    const {
        disallow_anonymous,
        followers_count,
        following_count,
        friends_count,
        hide_social,
        is_following,
        is_following_you,
        is_helper,
        is_powerful,
        is_self,
        layout,
        lock_profile,
        other,
        relationship,
        require_account
    } = page_data;

    const banner_fit = other.metadata.kv["sparkler:banner_fit"];
    const header = other.metadata.kv["sparkler:motivational_header"];
    const display_name = other.metadata.kv["sparkler:display_name"];
    const sidebar = other.metadata.kv["sparkler:sidebar"];
    const biography = other.metadata.kv["sparkler:biography"];

    onMount(() => {
        if (search_query.reply_intent) {
            const form = document.getElementById("question_form");

            if (form) {
                form.innerHTML += `<p class="fade">Replying to <a href="/response/${search_query.reply_intent}" target="_blank">${search_query.reply_intent}</a> (<a href="?" class="red">cancel</a>)</p>`;
            }
        }
    });

    // functions
    function follow() {
        fetch(`/api/v0/auth/relationships/follow/${other.username}`, {
            method: "POST"
        })
            .then((res) => res.json())
            .then(() => {
                // swap button
                const button = document.getElementById("follow_button") as HTMLElement;

                if (button.innerText === "Follow") {
                    button.classList.remove("primary");
                    button.classList.remove("bold");
                    button.innerText = "Unfollow";

                    trigger("app:toast", ["success", "User followed!"]);
                } else {
                    button.classList.add("primary");
                    button.classList.add("bold");
                    button.innerText = "Follow";

                    trigger("app:toast", ["success", "User unfollowed!"]);
                }
            });
    }
</script>

<svelte:head>
    <title>{other.username}</title>
    <meta name="description" content={biography} />
    <meta name="og:description" content={biography} />

    <meta name="og:title" content={other.username} />
    <meta name="og:url" content="{config.host}}/@{other.username}" />

    <meta property="og:type" content="profile" />
    <meta property="profile:username" content={other.username} />

    <meta name="og:image" content="{config.host}/api/v0/auth/profile/{other.id}/avatar" />

    <meta name="twitter:image" content="{config.host}/api/v0/auth/profile/{other.id}/avatar" />

    <meta name="twitter:card" content="summary" />
    <meta name="twitter:title" content="Ask me something!" />
    <meta name="twitter:description" content="Ask @{other.username} something on {config.name}!" />
</svelte:head>

<article class="flex flex-col gap-4">
    <style>
        footer {
            display: none !important;
        }
    </style>

    <button
        onclick={() => {
            (document.getElementById("search_dialog") as HTMLDialogElement).showModal();
        }}
        title="Search"
        class="primary floating right"
    >
        <Search class="icon" />
    </button>

    <div id="is_profile_page"></div>
    {#if banner_fit}
        <img
            title="{other.username}'s banner"
            src="/api/v0/auth/profile/{other.id}/banner"
            alt=""
            class="shadow round banner {banner_fit}"
            style="
               width: 100%;
               min-height: 150px;
               max-height: 440px;
           "
        />
    {:else}
        <img
            title="{other.username}'s banner"
            src="/api/v0/auth/profile/{other.id}/banner"
            alt=""
            class="shadow round banner cover"
            style="
               width: 100%;
               min-height: 150px;
               max-height: 440px;
               object-fit: cover;
           "
        />
    {/if}

    <div id="profile_box" class="flex flex-collapse gap-4 {layout === '1' ? 'flex-rev-row' : ''}">
        <div class="flex flex-col gap-4 sm:w-full profile_container" style="width: 25rem; height: max-content">
            <style>
                .profile_avatar {
                    --size: 160px;
                }

                .profile_avatar_container {
                    margin: -80px auto 0;
                }

                @media screen and (max-width: 900px) {
                    .profile_avatar {
                        --size: 120px;
                    }

                    .profile_avatar_container {
                        margin: -40px auto 0;
                    }
                }
            </style>

            <div
                id="profile_card"
                class="card shadow padded flex flex-col gap-2 w-full"
                style="padding-top: 0; height: max-content"
            >
                <div class="flex flex-col gap-2 profile_card_section_1">
                    <div class="flex flex-col gap-2 w-full">
                        <div style="position: relative" class="profile_avatar_container">
                            <img
                                title="{other.username}'s avatar"
                                src="/api/v0/auth/profile/{other.id}/avatar"
                                alt=""
                                class="avatar shadow-md profile_avatar"
                            />
                        </div>

                        <div id="names">
                            <div class="flex gap-2 items-center" style="max-width: 100%">
                                <h3 class="no-margin username">
                                    {#if display_name}
                                        {display_name}
                                    {:else}
                                        {other.username}
                                    {/if}
                                </h3>

                                <UserNote user={other} current_profile={user} {lang} use_static={true} />
                            </div>

                            <h4 class="no-margin username" style="font-weight: normal; opacity: 50%">
                                {other.username}
                            </h4>

                            {#if is_following_you}
                                <span class="notification notif-invert ff-inherit fs-md bold">Follows you</span>
                            {/if}

                            <div class="flex flex-wrap w-full gap-2">
                                {#each other.badges as badge}
                                    <span
                                        class="notification ff-inherit fs-md bold flex items-center justify-center"
                                        style="background: {badge[1]}; color: {badge[2]}; gap: 5px"
                                    >
                                        <Award class="icon" />
                                        {badge[0]}
                                    </span>
                                {/each}

                                {#if other.tier >= config.tiers.profile_badge}
                                    <span
                                        class="notification ff-inherit fs-md bold flex items-center justify-center"
                                        style="background: var(--color-primary); color: var(--color-text-primary); gap: 5px"
                                    >
                                        <Crown class="icon" />
                                        Supporter
                                    </span>
                                {/if}

                                {#if other.group === -1}
                                    <span
                                        class="notification ff-inherit fs-md bold flex items-center justify-center"
                                        style="background: var(--color-lowered); color: var(--color-text-lowered); gap: 5px"
                                    >
                                        <ShieldBan class="icon" />
                                        Banned
                                    </span>
                                {/if}
                            </div>
                        </div>
                    </div>

                    <div class="flex flex-col gap-2 profile_card_section_1_1">
                        <div id="biography">
                            {render_markdown(other.metadata.kv["sparkler:biography"] || "")}
                        </div>

                        {#if sidebar}
                            <div id="sidebar" class="card secondary shadow w-full">
                                {render_markdown(sidebar)}
                            </div>
                        {/if}

                        <!-- social -->
                        {#if !hide_social}
                            <div class="footernav flex-wrap justify-center profile_social" style="font-size: 13px;">
                                <a href="/@{other.username}/followers" class="item" style="color: var(--color-text)">
                                    <b>{followers_count}</b>
                                    <span class="fade"
                                        >{lang["profile:base.html:link.follower"]}{#if followers_count > 1}s{/if}</span
                                    >
                                </a>

                                <a href="/@{other.username}/following" class="item" style="color: var(--color-text)">
                                    <b>{following_count}</b>
                                    <span class="fade">{lang["profile:base.html:link.following"]}</span>
                                </a>

                                <a href="/@{other.username}/friends" class="item" style="color: var(--color-text)">
                                    <b>{friends_count}</b>
                                    <span class="fade"
                                        >{lang[
                                            "profile:base.html:link.friend"
                                        ]}{#if friends_count > 1 || friends_count === 0}s{/if}</span
                                    >
                                </a>
                            </div>
                        {/if}
                    </div>
                </div>

                <div class="flex flex-col gap-2 profile_card_section_2">
                    <!-- buttons -->
                    {#if user.is_some()}
                        {@const profile = user.unwrap()}
                        {#if profile.username === other.username}
                            <!-- options for account owner -->
                            <!-- <hr /> -->
                            <a title="Edit Profile" class="button w-full bold primary" href="/settings/profile">
                                <span class="possible_text">{lang["profile:base.html:link.edit_profile"]}</span>
                                <Pen class="icon" />
                            </a>

                            <Dropdown>
                                <button title="More" class="w-full">
                                    <span class="possible_text">{lang["general:link.more"]}</span>
                                    <ChevronDown class="icon dropdown-arrow" />
                                </button>

                                <div class="inner w-content left">
                                    <a href="/settings">
                                        <Settings class="icon" />
                                        {lang["profile:base.html:link.account_settings"]}
                                    </a>

                                    <button
                                        onclick={() => {
                                            (document.getElementById("embed_dialog") as HTMLDialogElement).showModal();
                                        }}
                                    >
                                        <Code class="icon" />
                                        {lang["profile:base.html:link.embed_profile"]}
                                    </button>
                                </div>
                            </Dropdown>
                        {:else}
                            <div class="flex gap-2">
                                <!-- follow, unfollow -->
                                {#if !is_following}
                                    <button
                                        class="w-full bold primary"
                                        onclick={() => {
                                            follow();
                                        }}
                                        id="follow_button"
                                    >
                                        {lang["profile:base.html:action.follow"]}
                                    </button>
                                {:else}
                                    <button
                                        class="w-full"
                                        onclick={() => {
                                            follow();
                                        }}
                                        id="follow_button"
                                    >
                                        {lang["profile:base.html:action.unfollow"]}
                                    </button>
                                {/if}

                                {#if relationship === "Unknown"}
                                    <button
                                        class="w-full primary bold"
                                        onclick={() => {
                                            fetch(`/api/v0/auth/relationships/friend/${other.id}`, {
                                                method: "POST"
                                            })
                                                .then((res) => res.json())
                                                .then((res) => {
                                                    trigger("app:toast", [
                                                        res.success ? "success" : "error",
                                                        res.success ? "Friend request sent!" : res.message
                                                    ]);
                                                });
                                        }}
                                    >
                                        {lang["profile:base.html:action.friend"]}
                                    </button>
                                {:else if relationship === "Friends"}
                                    <button
                                        class="w-full"
                                        onclick={async () => {
                                            if (
                                                !(await trigger("app:confirm", ["Are you sure you want to do this?"]))
                                            ) {
                                                return;
                                            }

                                            fetch(`/api/v0/auth/relationships/current/${other.id}`, {
                                                method: "DELETE"
                                            })
                                                .then((res) => res.json())
                                                .then((res) => {
                                                    trigger("app:toast", [
                                                        res.success ? "success" : "error",
                                                        res.success ? "User unfriended!" : res.message
                                                    ]);
                                                });
                                        }}
                                    >
                                        {lang["profile:base.html:action.unfriend"]}
                                    </button>
                                {:else if relationship === "Pending"}
                                    <button
                                        class="w-full"
                                        onclick={async () => {
                                            if (
                                                !(await trigger("app:confirm", ["Are you sure you want to do this?"]))
                                            ) {
                                                return;
                                            }

                                            fetch(`/api/v0/auth/relationships/current/${other.id}`, {
                                                method: "DELETE"
                                            })
                                                .then((res) => res.json())
                                                .then((res) => {
                                                    trigger("app:toast", [
                                                        res.success ? "success" : "error",
                                                        res.success ? "Request cancelled!" : res.message
                                                    ]);

                                                    window.close();
                                                });
                                        }}
                                        title="Cancel friend request"
                                    >
                                        {lang["general:dialog.cancel"]}
                                    </button>
                                {/if}
                            </div>

                            <!-- actions -->
                            <Dropdown>
                                <button class="w-full">
                                    <span class="possible_text">Actions</span>
                                    <ChevronDown class="icon dropdown-arrow" />
                                </button>

                                <div class="inner w-content left">
                                    <b class="title">This user</b>
                                    <button
                                        onclick={() => {
                                            trigger("chats:create", [other.id]);
                                        }}
                                    >
                                        <MessageCirclePlus class="icon" />
                                        {lang["general:link.chat"]}
                                    </button>
                                    <a href="/inbox/mail/compose?to={other.id}">
                                        <MailPlus class="icon" />
                                        {lang["general:service.mail"]}
                                    </a>
                                    <a href="/settings?block={other.username}#sparkler:block_somebody" target="_blank">
                                        <Shield class="icon" />
                                        {lang["general:action.block"]}
                                    </a>
                                    <button
                                        onclick={() => {
                                            trigger("reports:bootstrap", ["profiles", other.username]);
                                        }}
                                    >
                                        <Flag class="icon" />
                                        {lang["general:action.report"]}
                                    </button>
                                    <button
                                        onclick={() => {
                                            trigger("app:copy_text", [other.id]);
                                        }}
                                    >
                                        <Copy class="icon" />
                                        {lang["general:action.copy_id"]}
                                    </button>
                                    {#if is_powerful}
                                        <!-- for managers ONLY -->
                                        <button
                                            onclick={(e) => {
                                                if (!confirm("Are you sure you want to do this?")) {
                                                    return;
                                                }

                                                fetch(`/api/v0/auth/profile/${other.id}`, {
                                                    method: "DELETE"
                                                })
                                                    .then((res) => res.json())
                                                    .then((res) => {
                                                        trigger("app:shout", [
                                                            res.success ? "tip" : "caution",
                                                            res.message ||
                                                                "Profile deleted! Thanks for keeping {{ config.name }} clean!"
                                                        ]);

                                                        (e.target as any).reset();
                                                    });
                                            }}
                                        >
                                            <Trash class="icon" />
                                            {lang["general:action.delete"]}
                                        </button>
                                    {/if}
                                    <b class="title">Your account</b>
                                    <a href="/settings#sparkler:blocks">
                                        <Lock class="icon" />
                                        {lang["profile:base.html:link.manage_blocks"]}
                                    </a>
                                </div>
                            </Dropdown>
                        {/if}
                    {:else}
                        <!-- anonymous actions -->
                        <Dropdown>
                            <button class="w-full">
                                {lang["general:link.actions"]}
                                <ChevronDown class="icon dropdown-arrow" />
                            </button>

                            <div class="inner w-content left">
                                <b class="title">{lang["profile:base.html:title.this_user"]}</b>
                                <button
                                    onclick={() => {
                                        trigger("reports:bootstrap", ["profiles", other.username]);
                                    }}
                                >
                                    <Flag class="icon" />
                                    {lang["general:action.report"]}
                                </button>
                            </div>
                        </Dropdown>
                    {/if}
                </div>
            </div>

            <hr class="mobile small" />
        </div>

        <!-- locked message -->
        {#if (relationship != "Friends" && other.metadata["sparkler:private_profile"] === "true") || other.group === -1}
            <div class="card padded shadow flex flex-col w-full gap-4 items-center justify-center">
                <LockKeyhole class="icon" />
                <h4>{lang["profile:base.html:text.private"]}</h4>
            </div>
        {:else}
            <section id="feed" class="flex flex-col gap-4 w-full">
                <!-- upper -->
                <!-- new question -->
                <div class="card-nest w-full shadow" id="question_box">
                    <div class="card motivational_header">
                        {#if header}
                            {render_markdown(header)}
                        {:else}
                            Ask a question
                        {/if}
                    </div>

                    <div class="card">
                        {#if !lock_profile && other.group != -1}
                            {#if (require_account && profile.is_some()) || (disallow_anonymous && profile.is_some()) || (!require_account && !disallow_anonymous)}
                                <form
                                    id="question_form"
                                    class="flex flex-col gap-2"
                                    onsubmit={(e) => {
                                        e.preventDefault();
                                        trigger("questions:create", [
                                            other.id,
                                            search_query.reply_intent
                                                ? `${(e.target as any).content.value}\n\n/+r/${search_query.reply_intent}`
                                                : (e.target as any).content.value,
                                            (
                                                (e.target as any).anonymous || {
                                                    checked: false
                                                }
                                            ).checked,
                                            (e.target as any).carp_content.value.length != 0
                                                ? (e.target as any).carp_content.value
                                                : ""
                                        ]).then(() => {
                                            // reset if successful
                                            (e.target as any).reset();

                                            if ((globalThis as any).sammy) {
                                                (globalThis as any).sammy.clear();
                                            }

                                            if ((globalThis as any).ls_anon_check) {
                                                (globalThis as any).ls_anon_check();
                                            }
                                        });
                                    }}
                                >
                                    <div id="carp_context"></div>
                                    <input name="carp_context" id="carp_content" type="text" style="display: none" />

                                    <textarea
                                        class="w-full"
                                        placeholder="Type your question!"
                                        minlength="1"
                                        maxlength="2048"
                                        required
                                        name="content"
                                        id="content"
                                        data-hook="counter"
                                    ></textarea>

                                    <div class="flex justify-between w-full gap-1 flex-wrap">
                                        <div class="footernav items-center gap-2">
                                            <span id="content:counter" class="notification item"></span>
                                            {#if user.is_some() && disallow_anonymous === false}
                                                <div class="checkbox_container item">
                                                    <input type="checkbox" name="anonymous" id="anonymous" />

                                                    <label for="anonymous" class="normal">
                                                        {lang["general:action.hide_your_name"]}
                                                    </label>
                                                </div>
                                            {:else}
                                                <div></div>
                                            {/if}
                                        </div>

                                        <div class="flex gap-2">
                                            {#if other.metadata["sparkler:allow_drawings"] === "true"}
                                                <button
                                                    onclick={(e) => {
                                                        (e.target as any).innerText = "Remove drawing";
                                                        (e.target as any).onclick = async (e: any) => {
                                                            if (
                                                                !(await trigger("app:confirm", [
                                                                    "Are you sure you want to do this?"
                                                                ]))
                                                            ) {
                                                                return;
                                                            }

                                                            e.target.innerText = "Draw";
                                                            e.target.onclick = (e: any) => {
                                                                (globalThis as any).attach_carp(e);
                                                            };

                                                            (
                                                                document.getElementById(
                                                                    "carp_context"
                                                                ) as HTMLInputElement
                                                            ).innerHTML = "";
                                                            (
                                                                document.getElementById(
                                                                    "carp_content"
                                                                ) as HTMLInputElement
                                                            ).value = "";
                                                            (globalThis as any).sammy = null;
                                                        };

                                                        use("carp", (carp: any) => {
                                                            const sammy = carp.new(
                                                                document.getElementById("carp_context")
                                                            );

                                                            sammy.create_canvas();
                                                            sammy.onedit = (text: string) => {
                                                                (
                                                                    document.getElementById(
                                                                        "carp_content"
                                                                    ) as HTMLInputElement
                                                                ).value = `--CARP${text}`;
                                                            };

                                                            (globalThis as any).sammy = sammy;
                                                        });
                                                    }}
                                                    type="button">Draw</button
                                                >
                                            {/if}

                                            <button class="primary bold">
                                                {lang["profile:base.html:action.ask"]}
                                            </button>
                                        </div>
                                    </div>
                                </form>
                            {:else}
                                <b>{lang["profile:base.html:text.no_anonymous_questions"]}</b>
                            {/if}
                        {:else}
                            <b>{lang["profile:base.html:text.no_questions"]}</b>
                        {/if}
                    </div>
                </div>

                <!-- panel -->
                <div id="panel" style="display: contents">
                    {@render children()}
                </div>
            </section>
        {/if}
    </div>
</article>
