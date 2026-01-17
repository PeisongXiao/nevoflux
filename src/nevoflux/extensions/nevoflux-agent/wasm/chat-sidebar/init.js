import init, * as bindings from './chat-sidebar-93e7aaae9f68f44f.js';
const wasm = await init({ module_or_path: './chat-sidebar-93e7aaae9f68f44f_bg.wasm' });


window.wasmBindings = bindings;


dispatchEvent(new CustomEvent("TrunkApplicationStarted", {detail: {wasm}}));