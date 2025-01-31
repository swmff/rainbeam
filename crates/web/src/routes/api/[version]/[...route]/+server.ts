import type { RequestHandler } from "./$types";
import * as db from "$lib/db";

export const fallback: RequestHandler = async ({ request, params, url }) => {
    const res = await db.api.req(
        request.method,
        { version: params.version, route: params.route + url.search },
        { headers: request.headers, body: request.body }
    );

    return new Response(await res.arrayBuffer(), {
        status: res.status,
        headers: res.headers
    });
};
