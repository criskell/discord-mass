const api = globalThis.browser ?? globalThis.chrome;

wasm_bindgen({ module_or_path: api.runtime.getURL("pkg/discord_mass_bg.wasm") })
  .then(() => {
    api.runtime.onMessage.addListener(() => {
      wasm_bindgen.toggle_panel();
    });
  })
  .catch((error) => {
    console.error("[discord-mass] falha ao carregar o wasm", error);
  });
