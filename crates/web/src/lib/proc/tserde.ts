//! TypeScript serialization decorators
export type Serialized = {
    [key: string]: any;
};

/* export function Serialize(target_fields: Array<string>) {
    return function decorator(_: any, context: ClassMethodDecoratorContext) {
        return function (this: any): Serialized {
            const output_object: Serialized = {};

            for (const field of target_fields) {
                output_object[field] = this[field];
            }

            return output_object;
        };
    };
}

export function Deserialize(target_fields: Array<string>) {
    return function decorator(_: any, context: ClassMethodDecoratorContext) {
        return function (this: any, serialized: Serialized) {
            for (const field of target_fields) {
                this[field] = serialized[field];
            }
        };
    };
} */

export function Serialize() {
    return function (target: any, _: string) {
        target.serialize_into = (
            fields: Serialized,
            target_fields: Array<string>
        ) => {
            const output_object: Serialized = {};

            for (const field of target_fields) {
                output_object[field] = fields[field];
            }

            return output_object;
        };
    };
}
