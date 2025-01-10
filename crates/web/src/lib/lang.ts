import type { LangFile as LangFileStruct } from "./bindings/LangFile";

const langs: { [key: string]: LangFileStruct } = {};

export async function get(id: string): Promise<LangFileStruct> {
    const file: LangFileStruct = await Bun.file(`./langs/${id}.json`).json();
    langs[id] = file;
    return file;
}

await get("en-US");
await get("ko-KR");

export default function text(lang: string, key: string): string {
    return langs[lang].data[key] || key;
}
