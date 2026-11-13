// @target: esnext
// @module: preserve
// @moduleResolution: bundler
// @jsx: react-jsx
// @strict: true

// Test for unicode escape sequence issue in JSX
// The warning symbol (⚠, U+26A0) should be corrA0, not \uFFFD

// @filename: jsxUnicodeEscapeSequence.tsx

/// <reference path="/.lib/react16.d.ts" />

export const InlineUnicodeChar = () => {
    // This should work correctly - inline character with other content
    return <div><span>Warning: ⚠ Error</span></div>;
};

export const StandaloneUnicodeChar = () => {
    // This should reproduce the issue - unicode character on its own line
    return (<div><span>⚠</span>
        ⚠
    </div>);
};

export const MultipleUnicodeChars = () => {
    // Test multiple unicode characters
    return (<div>
        ⚠
        ⛔
        🚨
    </div>);
};
