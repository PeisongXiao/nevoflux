import init, * as bindings from './chat-sidebar-cb59ac12d5236340.js';
const wasm = await init({ module_or_path: './chat-sidebar-cb59ac12d5236340_bg.wasm' });


window.wasmBindings = bindings;


dispatchEvent(new CustomEvent("TrunkApplicationStarted", {detail: {wasm}}));