rag Fragment
 */
import {h, Fragment} from "./renderer";
<></>

// @filename: snabbdomy-only-fragment.tsx
/* @jsx jsx */
/* @jsxfrag null */
import {jsx} from "./renderer";
<></>

// @filename: preacty-only-fragment-no-jsx.tsx
/**
 * @jsx h
 * @jsxFrag Fragment
 */
import {Fragment} from "./renderer";
<></>

// @filename: snabbdomy-only-fragment-no-jsx.tsx
/* @jsx jsx */
/* @jsxfrag null */
import {} from "./renderer";
<></>

// @filename: preacty-no-fragment.tsx
/**
 * @jsx h
 * @jsxFrag Fragment
 */
import {h, Fragment} from "./renderer";
<div></div>

// @filename: snabbdomy-no-fragment.tsx
/* @jsx jsx */
/* @jsxfrag null */
import {jsx} from "./renderer";
<div></div>

// @filename: preacty-only-component.tsx
/**
 * @jsx h
 */
import {h} from "./renderer";
function Component() { return null; }
<Component />
