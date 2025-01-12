import type { LayoutServerLoad } from "./$types";
import { type Option, Some, None } from "$lib/classes/Option";
import type { Profile } from "$lib/bindings/Profile";

import { langs } from "$lib/lang";
import * as db from "$lib/db";
import type { LangFile } from "$lib/bindings/LangFile";
import type { Serialized } from "$lib/proc/tserde";

export type LayoutData = {
    user: Serialized;
    notifs: number;
    unread: number;
    lang: LangFile["data"];
    config: db.CleanConfig;
    data: any;
    query: Serialized;
};

export const load: LayoutServerLoad = async ({
    cookies,
    url,
    request
}): Promise<LayoutData> => {
    const token = cookies.get("__Secure-Token");
    const lang = cookies.get("net.rainbeam.langs.choice");

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

    // return
    return {
        user: token
            ? Some(
                  (await db.get_profile_from_token(token)).payload as Profile
              ).serialize()
            : (None as Option<Profile>).serialize(),
        notifs: unread.payload[0],
        unread: unread.payload[1],
        lang: langs[lang || "net.rainbeam.langs:en-US"].data,
        config: {
            name: db.config.name,
            description: db.config.description,
            host: db.config.host,
            captcha: {
                site_key: db.config.captcha.site_key
            }
        },
        data: data.status === 200 ? (await data.json()).payload : {},
        query
    };
};
