import type { LayoutServerLoad } from "./$types";
import { type Option, Some, None } from "$lib/classes/Option";
import type { Profile } from "$lib/bindings/Profile";

import { langs } from "$lib/lang";
import * as db from "$lib/db";

export const load: LayoutServerLoad = async ({ cookies, url, request }) => {
    const token = cookies.get("__Secure-Token");
    const lang = cookies.get("net.rainbeam.langs.choice");

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
        lang: langs[lang || "net.rainbeam.langs:en-US"].data,
        config: db.config,
        data: data.status === 200 ? (await data.json()).payload : {}
    };
};
