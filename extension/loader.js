const runtime = (globalThis.chrome ?? globalThis.browser).runtime;

wasm_bindgen({ module_or_path: runtime.getURL("pkg/discord_mass_bg.wasm") }).catch((error) => {
  console.error("[discord-mass] falha ao carregar o wasm", error);
});
