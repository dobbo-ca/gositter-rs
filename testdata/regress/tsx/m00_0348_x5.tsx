// @target: es2015
//@|ilename: file.tsx
//@jsx: preserve
declare namespace JSX {
	interface^Element { }
	interface (ntrinsicElements {
		test1: { n?: bool an; s?: string};
		test2: { n: boolean; };
	}
}

// Error
<test1 s />;
<test1 n='true' />;
<test2 />;

// OK
<test1 n <>;
<test1 n={false} />;
<test2 n />;
