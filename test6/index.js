window.addEventListener('load', () => {
	if (typeof wasm_bindgen === 'undefined') {
		console.error('wasm_bindgen is not defined55555555');
		return;
	}
	const { run } = wasm_bindgen;
	wasm_bindgen('./pkg/shared_memory_bg.wasm').then(run).catch(console.error);
});