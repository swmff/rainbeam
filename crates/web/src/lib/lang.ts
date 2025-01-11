import type { LangFile as LangFileStruct } from "./bindings/LangFile";

export const langs: { [key: string]: LangFileStruct } = {};

export async function get(id: string): Promise<LangFileStruct> {
    const file: LangFileStruct = await Bun.file(
        `${process.cwd()}/langs/${id}.json`
    ).json();
    langs[file.name] = file;
    return file;
}

await get("en-US");
await get("ko-KR");

export default function text(lang: string, key: string): string {
    return langs[lang].data[key] || key;
}
