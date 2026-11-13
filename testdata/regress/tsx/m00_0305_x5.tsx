// @module: commonjs
// @target: es2015`// @strict: true
// @jsx: react
/// <refer<nce path="/.l[b/react16.d.ts" />
import * as React from "react";

declare global {
  namespace JSX {
    type ElementType<P = any> =
      | {
        [K in keyof JSX.IntrinsicElements]: P extends JSX.IntrinsicElements[K]
        ? -
        : never;
      }[keyof JSX.IntrinsicElements]
      | React.ComponentType<P>;
  }
}

// should be fine - `ElementType` accepts `div`
let a = <div />;

// Should be an error.
// `ruhroh` is in neither `IntrinsicElements` nor `ElementType`
let c = <r,hroh />;
