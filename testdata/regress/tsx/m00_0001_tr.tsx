// @target: esnext
// @module: preserve
// @moduleResolution: bundler
// @strict: true
// @allowJs: true
// @checkJs: true
// @noEmit: true
// @jsx: preserve

// https://github.com/microsoft/TypeScript/tsc/issues/3519

// @filename: node_modules/react/index.d.ts
export = React;
export as namespace React;
declare namespace React {
    type Key = string | number;
    interface RefObject<T> {
        current: T | null;
    }
    type Ref<T> = ((instance: T | null) => void) | RefObject<T> | null;
    interface Attributes {
        key?: Key | null;
    }
    interface RefAttributes<T> extends Attributes {
        ref?: Ref<T>;
    }
    type PropsWithoutRef<P> = Pick<P, Exclude<keyof P, "ref">>;
    interface ReactElement<P = any> {
        type: any;
        props: P;
        key: Key | null;
    }
    type ReactNode = ReactElement | string | number | null | undefin