import { config } from "$lib/db";

export type ApiVersion = "v0" | "v1";
export enum ApiMethod {
    Get = "GET",
    Post = "POST",
    Put = "PUT",
    Delete = "DELETE"
}

export type ApiParams = {
    version: ApiVersion | string;
    route: string;
};

export type ApiRequest = {
    headers: { [key: string]: string } | Headers;
    body: ReadableStream<Uint8Array<ArrayBufferLike>> | null;
};

export default class ApiProxy {
    public async req(
        method: ApiMethod | string,
        params: ApiParams,
        request: ApiRequest
    ) {
        return await fetch(
            `${config.client.api}/api/${params.version}/${params.route}`,
            {
                method,
                headers: request.headers,
                body: request.body
            }
        );
    }

    public async req_root(
        method: ApiMethod | string,
        params: ApiParams,
        request: ApiRequest
    ) {
        return await fetch(`${config.client.api}/${params.route}`, {
            method,
            headers: request.headers,
            body: request.body
        });
    }

    public async get(params: ApiParams, request: ApiRequest) {
        return await this.req(ApiMethod.Get, params, request);
    }

    public async post(params: ApiParams, request: ApiRequest) {
        return await this.req(ApiMethod.Post, params, request);
    }

    public async put(params: ApiParams, request: ApiRequest) {
        return await this.req(ApiMethod.Put, params, request);
    }

    public async delete(params: ApiParams, request: ApiRequest) {
        return await this.req(ApiMethod.Delete, params, request);
    }

    public async get_root(params: ApiParams, request: ApiRequest) {
        return await this.req_root(ApiMethod.Get, params, request);
    }

    public async post_root(params: ApiParams, request: ApiRequest) {
        return await this.req_root(ApiMethod.Post, params, request);
    }

    public async put_root(params: ApiParams, request: ApiRequest) {
        return await this.req_root(ApiMethod.Put, params, request);
    }

    public async delete_root(params: ApiParams, request: ApiRequest) {
        return await this.req_root(ApiMethod.Delete, params, request);
    }
}
