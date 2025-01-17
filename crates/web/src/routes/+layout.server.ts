import type { LayoutServerLoad } from "./$types";
import { Option, Some } from "$lib/classes/Option";
import type { Profile } from "$lib/bindings/Profile";

import { langs } from "$lib/lang";
import * as db from "$lib/db";
import type { LangFile } from "$lib/bindings/LangFile";
import type { Serialized } from "$lib/proc/tserde";
import { BAD_ITEMS, BAD_ITEMS_POWERFUL, clean, aclosure } from "$lib/helpers";
import { error } from "@sveltejs/kit";

export type LayoutData = {
    user: Serialized;
    notifs: number;
    unread: number;
    lang: LangFile["data"];
    lang_name: LangFile["name"];
    config: db.CleanConfig;
    data: any;
    query: Serialized;
    layout_skip: boolean;
};

export const load: LayoutServerLoad = async ({ cookies, url, request }): Promise<LayoutData> => {
    if (url.pathname.startsWith("/_")) {
        // @ts-ignore
        return { user: { inner: null }, layout_skip: true };
    }

    const token = cookies.get("__Secure-Token");
    const lang = cookies.get("net.rainbeam.langs.choice");

    // get user
    const user = await aclosure(async () => {
        if (token) {
            return Some((await db.get_profile_from_token(token)).payload as Profile);
        }

        return Option.None(); // for some reason just using None here will return a random profile...
    });

    if (
        user.is_none() &&
        (url.pathname.startsWith("/inbox") ||
            url.pathname === "/intents/post" ||
            url.pathname.startsWith("/settings") ||
            url.pathname.startsWith("/market") ||
            url.pathname.startsWith("/mail") ||
            url.pathname.startsWith("/chats"))
    ) {
        throw error(401, "Unauthorized");
    }

    // build query params map
    let query: Serialized = {};

    for (const param of Array.from(url.searchParams.entries())) {
        query[param[0]] = param[1];
    }

    // fetch unread data
    const unread = await db.get_unread(request.headers);

    // fetch page data
    const data = await db.api.get_root(
        {
            route: url.pathname + url.search,
            version: ""
        },
        {
            body: null,
            headers: request.headers
        }
    );

    const payload = data.status === 200 ? (await data.json()).payload : {};

    // return
    return {
        user: user.serialize(),
        notifs: (unread.payload || [0, 0])[1],
        unread: (unread.payload || [0, 0])[0],
        lang: (langs[lang || "net.rainbeam.langs:en-US"] || langs["net.rainbeam.langs:en-US"]).data,
        lang_name: lang || "net.rainbeam.langs:en-US",
        config: {
            name: db.config.name,
            description: db.config.description,
            host: db.config.host,
            tiers: db.config.tiers,
            captcha: {
                site_key: db.config.captcha.site_key
            }
        },
        data:
            (payload || { is_powerful: false }).is_powerful === true
                ? clean(payload, BAD_ITEMS_POWERFUL)
                : clean(payload, BAD_ITEMS),
        query,
        layout_skip: false
    };
};
