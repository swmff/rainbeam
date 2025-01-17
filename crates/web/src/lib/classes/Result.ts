import { Serialize, type Serialized } from "$lib/proc/tserde";
import { None, Some, type Option, type Value } from "./Option";

export class Result<T> {
    private inner: Value<T>;
    private inner_is_err: boolean;

    constructor(inner: Value<T>, err = false) {
        this.inner = inner;
        this.inner_is_err = err;
    }

    public static Ok<T>(inner: Value<T>) {
        return new Result<T>(inner);
    }

    public static Err(message: string) {
        return new Result<string>(message, true);
    }

    public is_some() {
        return this.inner_is_err === false;
    }

    public is_err() {
        return this.inner_is_err === true;
    }

    public unwrap(): Value<T> {
        if (this.is_err()) {
            console.error("Attempted to unwrap an Err value");
            return null;
        }

        return this.inner;
    }

    public err(): Option<string> {
        if (this.is_err()) {
            return Some(this.inner as string);
        }

        return None;
    }

    // traits
    @Serialize()
    public serialize(): Serialized {
        return (this as any).serialize_into(this, ["inner", "inner_is_err"]);
    }

    public static from(serialized: Serialized) {
        const c = new Result<any>(null, true);
        c.inner = serialized.inner;
        c.inner_is_err = serialized.inner_is_err;
        return c;
    }
}

export const Ok = <T>(inner: Value<T>) => Result.Ok<T>(inner);
export const Err = (message: string) => Result.Err(message);
