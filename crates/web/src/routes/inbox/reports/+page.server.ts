import { error } from "@sveltejs/kit";
import type { PageServerLoad } from "./$types";
import { Option } from "$lib/classes/Option";

export const load: PageServerLoad = async ({ parent }) => {
    const par = await parent();
    const user = Option.from(par.user);

    if (!par.data || !user.is_none() || user.unwrap().id !== par.data.other.id) {
        if (!par.data || !par.data.is_helper) {
            throw error(401, "Cannot view moderator page without moderator permissions.");
        }
    }

    return {};
};
