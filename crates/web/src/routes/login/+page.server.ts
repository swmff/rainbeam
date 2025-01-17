import { error } from "@sveltejs/kit";
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = async ({ cookies }) => {
    const token = cookies.get("__Secure-Token");

    if (token) {
        throw error(401, "Already authenticated.");
    }

    return {};
};
