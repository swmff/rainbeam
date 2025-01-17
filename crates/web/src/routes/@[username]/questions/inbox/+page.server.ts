import { error } from "@sveltejs/kit";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({ parent }) => {
    const par = await parent();

    if (!par.data || !par.data.is_helper) {
        throw error(401, "Cannot view moderator page without moderator permissions.");
    }

    return {};
};
