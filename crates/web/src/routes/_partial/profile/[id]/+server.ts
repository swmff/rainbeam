import type { RequestHandler } from "./$types";
import * as db from "$lib/db";

export const GET: RequestHandler = async ({ request, url, params }) => {
    const res = await db.api.get_root(
        {
            version: "",
            route: `/@${params.id}/_app/feed.html?clean=true&page=${url.searchParams.get("page") || "0"}`
        },
        { headers: request.headers, body: request.body }
    );

    return new Response(await res.text(), {
        status: res.status,
        headers: res.headers
    });
};
