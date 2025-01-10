import type { LayoutServerLoad } from "./$types";
import { type Option, Some, None } from "$lib/classes/Option";
import * as db from "$lib/db";
import type { Profile } from "$lib/bindings/Profile";

export const load: LayoutServerLoad = async ({ cookies }) => {
    const token = cookies.get("__Secure-Token");

    return {
        user: token
            ? Some(
                  (await db.get_profile_from_token(token)).payload as Profile
              ).serialize()
            : (None as Option<Profile>).serialize()
    };
};
