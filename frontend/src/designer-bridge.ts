// Designer fork: how a host page drives the editor.
//
// The editor is served from its own origin and shown in a frame by a host (the Designer dashboard). The host
// holds the person's session and the design; the editor holds nothing but what it is handed. They talk by
// `postMessage`:
//
//   host → editor   { designer: 1, id, method, params }
//   editor → host   { designer: 1, id, ok: true, result } | { designer: 1, id, ok: false, error }
//   editor → host   { designer: 1, event: "ready" }                       once, when it can take requests
//
// Methods (the Rust side is `editor/src/designer.rs`):
//
//   open          { name, canvas: { width, height, background }, layers: [{ ...layer, bytes }] }
//                 → { artboard, layers: [id, ...] }       a new document: one picture layer per layer given
//   openDocument  { name, bytes }                          a document saved by `getDocument`
//   describe      → { canvas, artboard, layers: [...] }    what the document holds now, bottom layer first
//   exportLayer   { id, scale } → { bounds, bytes }        one layer alone, as a PNG
//   getDocument   → { bytes }                              the document's file
//
// Only the page that frames the editor is answered, and answers go back to it alone. The same methods are on
// `window.designerGraphite.call(method, params)` for a page that hosts the editor directly.

import type { EditorWrapper } from "/wrapper/pkg/graphite_wasm_wrapper";

type Params = Record<string, unknown>;
type Described = { error?: string; layers?: { visible: boolean; bounds: number[] | null }[] } & Params;

const EXPORT_TIMEOUT = 120_000;
const SETTLE_TIMEOUT = 20_000;

/** Exports and saves the bridge asked for, by the file name they will arrive under. */
const awaited = new Map<string, (blob: Blob) => void>();
let exports = 0;

declare global {
	interface Window {
		/** Called by `downloadFileBlob`: true if the bridge was waiting for this file, so it is not downloaded. */
		designerCapture?: (filename: string, blob: Blob) => boolean;
		designerGraphite?: { call: (method: string, params?: Params) => Promise<unknown> };
	}
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

function expectFile(matches: (filename: string) => boolean, key: string): Promise<Blob> {
	return new Promise((resolve, reject) => {
		const timer = setTimeout(() => {
			awaited.delete(key);
			reject(new Error("the editor did not produce the file in time"));
		}, EXPORT_TIMEOUT);
		awaited.set(key, (blob) => {
			clearTimeout(timer);
			awaited.delete(key);
			resolve(blob);
		});
		matchers.set(key, matches);
	});
}
const matchers = new Map<string, (filename: string) => boolean>();

function parse(answer: string): Params {
	const value = JSON.parse(answer) as Params;
	if (typeof value.error === "string") throw new Error(value.error);
	return value;
}

export function installDesignerBridge(editor: EditorWrapper) {
	window.designerCapture = (filename, blob) => {
		for (const [key, deliver] of awaited) {
			if (matchers.get(key)?.(filename)) {
				matchers.delete(key);
				deliver(blob);
				return true;
			}
		}
		return false;
	};

	const describe = () => JSON.parse(editor.designerDescribe()) as Described;

	/** Bounds come from a render, which follows a change by a moment: wait until every shown layer has them. */
	async function settled(): Promise<Described> {
		const deadline = Date.now() + SETTLE_TIMEOUT;
		for (;;) {
			const now = describe();
			const ready = !now.error && (now.layers ?? []).every((layer) => !layer.visible || layer.bounds !== null);
			if (ready || Date.now() > deadline) return now;
			await sleep(100);
		}
	}

	const methods: Record<string, (params: Params) => Promise<{ result: unknown; transfer?: Transferable[] }>> = {
		async open(params) {
			const canvas = params.canvas as { width: number; height: number; background: string | null };
			const layers = params.layers as (Params & { bytes: ArrayBuffer })[];
			const artboard = editor.designerNewDocument(String(params.name ?? "Design"), canvas.width, canvas.height, canvas.background ?? "");
			const ids: string[] = [];
			for (const { bytes, ...layer } of layers) {
				const answer = editor.designerAddLayer(artboard, JSON.stringify(layer), new Uint8Array(bytes));
				if (answer.startsWith("{")) parse(answer);
				ids.push(answer);
			}
			// Fitting the view needs to know how large things are, which the first render tells.
			await settled();
			editor.designerShowAll();
			return { result: { artboard, layers: ids } };
		},

		async openDocument(params) {
			const name = String(params.name ?? "Design");
			editor.openFile(name.endsWith(".graphite") ? name : `${name}.graphite`, new Uint8Array(params.bytes as ArrayBuffer));
			// Opening is not immediate: the document's pictures are read first.
			const deadline = Date.now() + SETTLE_TIMEOUT;
			while (describe().error && Date.now() < deadline) await sleep(100);
			const now = await settled();
			if (now.error) throw new Error(now.error);
			editor.designerShowAll();
			return { result: {} };
		},

		async describe() {
			const now = await settled();
			if (now.error) throw new Error(now.error);
			return { result: now };
		},

		async exportLayer(params) {
			const name = `designer-layer-${++exports}`;
			const file = expectFile((filename) => filename === `${name}.png`, name);
			let started: Params;
			try {
				started = parse(editor.designerExportLayer(String(params.id), Number(params.scale ?? 1), name));
			} catch (error) {
				awaited.delete(name);
				matchers.delete(name);
				file.catch(() => undefined);
				throw error;
			}
			const bytes = await (await file).arrayBuffer();
			return { result: { bounds: started.bounds, bytes }, transfer: [bytes] };
		},

		async getDocument() {
			const file = expectFile((filename) => filename.endsWith(".graphite"), "document");
			editor.designerSaveDocument();
			const bytes = await (await file).arrayBuffer();
			return { result: { bytes }, transfer: [bytes] };
		},
	};

	// One request at a time, in the order asked: each works on what the one before left.
	let queue: Promise<unknown> = Promise.resolve();
	const call = (method: string, params: Params = {}) => {
		const run = async () => {
			const handler = methods[method];
			if (!handler) throw new Error(`the editor has no method "${method}"`);
			return handler(params);
		};
		const next = queue.then(run, run);
		queue = next.catch(() => undefined);
		return next;
	};

	window.designerGraphite = { call: async (method, params) => (await call(method, params)).result };

	const host = window.parent !== window ? window.parent : undefined;
	window.addEventListener("message", (event) => {
		const data = event.data as { designer?: number; id?: unknown; method?: unknown; params?: Params } | null;
		if (!host || event.source !== host || !data || data.designer !== 1 || typeof data.method !== "string") return;
		const { id, method, params } = data;
		call(method, params ?? {}).then(
			({ result, transfer }) => host.postMessage({ designer: 1, id, ok: true, result }, event.origin, transfer ?? []),
			(error: unknown) => host.postMessage({ designer: 1, id, ok: false, error: error instanceof Error ? error.message : String(error) }, event.origin),
		);
	});
	// Nothing is in this message but that the editor is up; whoever frames it may know that.
	host?.postMessage({ designer: 1, event: "ready" }, "*");
}
