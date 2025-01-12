import type { RequestHandler } from "./$types";
import * as db from "$lib/db";

export const GET: RequestHandler = async ({ request }) => {
    const res = await db.api.get_root(
        { version: "", route: "/_app/timelines/timeline.html?clean=true" },
        { headers: request.headers, body: request.body }
    );

    return new Response(await res.text(), {
        status: res.status,
        headers: res.headers
    });
};
