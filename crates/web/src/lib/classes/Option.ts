import { Serialize, type Serialized } from "$lib/proc/tserde";

export type None = null;
export type Value<T> = T | None;

export class Option<T> {
    private inner: Value<T>;

    constructor(inner: Value<T>) {
        this.inner = inner;
    }

    public static Some<T>(inner: Value<T>) {
        return new Option<T>(inner);
    }

    public static None() {
        return new Option<any>(null);
    }

    public is_some() {
        return this.inner !== null;
    }

    public is_none() {
        return this.inner === null;
    }

    public unwrap(): T {
        if (this.is_none()) {
            throw new Error("Attempted to unwrap a None value");
        }

        return this.inner as T;
    }

    // traits
    @Serialize()
    public serialize(): Serialized {
        return (this as any).serialize_into(this, ["inner"]);
    }

    public static from(serialized: Serialized) {
        const c = None;
        c.inner = serialized.inner;
        return c;
    }
}

export const Some = <T>(inner: Value<T>) => Option.Some<T>(inner);
export const None = Option.None();
