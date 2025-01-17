import type { Config } from "./bindings/Config";
import ApiProxy from "./classes/ApiProxy";
import toml from "smol-toml";

export type CleanConfig = {
    name: Config["name"];
    description: Config["description"];
    host: Config["host"];
    tiers: Config["tiers"];
    captcha: {
        site_key: Config["captcha"]["site_key"];
    };
};

export const config: Config = await (async () => {
    const file: string = await Bun.file("../../.config/config.toml").text();
    return toml.parse(file) as Config;
})();

export const api = new ApiProxy();

export async function get_profile(id: string) {
    return await (
        await api.get(
            {
                version: "v0",
                route: `auth/profile/${id}`
            },
            {
                body: null,
                headers: {}
            }
        )
    ).json();
}

export async function get_profile_from_token(token: string) {
    if (!token) {
        return {
            success: false,
            message: "",
            payload: null
        };
    }

    return await (
        await api.get(
            {
                version: "v0",
                route: `auth/token/${token}`
            },
            {
                body: null,
                headers: {}
            }
        )
    ).json();
}

export async function get_unread(headers: Headers) {
    return await (
        await api.get(
            {
                route: "profiles/me/unread",
                version: "v1"
            },
            {
                body: null,
                headers
            }
        )
    ).json();
}

export async function get_response(id: string) {
    return await (
        await api.get(
            {
                route: `responses/${id}`,
                version: "v1"
            },
            {
                body: null,
                headers: {}
            }
        )
    ).json();
}

export async function get_question(id: string) {
    return await (
        await api.get(
            {
                route: `questions/${id}`,
                version: "v1"
            },
            {
                body: null,
                headers: {}
            }
        )
    ).json();
}

export async function get_comment(id: string) {
    return await (
        await api.get(
            {
                route: `comments/${id}`,
                version: "v1"
            },
            {
                body: null,
                headers: {}
            }
        )
    ).json();
}
