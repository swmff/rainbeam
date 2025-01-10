import type { Config } from "./bindings/Config";
import ApiProxy from "./classes/ApiProxy";
import toml from "smol-toml";

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
