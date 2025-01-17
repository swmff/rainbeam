import { redirect } from "@sveltejs/kit";
import type { RequestHandler } from "./$types";
import * as db from "$lib/db";

export const fallback: RequestHandler = async ({ request, params }) => {
    const response = await db.get_comment(params.id);

    if (!response.success) {
        return redirect(303, "/");
    }

    return redirect(
        303,
        `/@${response.payload[0].author.username}/c/${params.id}`
    );
};
