md in this directory for provenance and licensing notes.
// Regenerate: npm run codex:gen-protocol
// Source file: v2/McpServerElicitationRequestResponse.ts

import type { JsonValue } from "../serde_json/JsonValue.js";
import type { McpServerElicitationAction } from "./McpServerElicitationAction.js";

export type McpServerElicitationRequestResponse = {
	action: McpServerElicitationAction,
	/**
	 * Structured user input for accepted elicitations, mirroring RMCP `CreateElicitationResult`.
	 *
	 * This is nullable because decline/cancel responses have no content.
	 */
	content: JsonValue | null,
	/**
	 * Optional client metadata for form-mode action handling.
	 */
	_meta: JsonValue | null,
};
