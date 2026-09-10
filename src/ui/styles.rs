pub const STYLES: &str = r#"
:host { all: initial; }
* { box-sizing: border-box; font-family: "gg sans", "Noto Sans", sans-serif; }

.panel {
  position: fixed;
  top: 72px;
  right: 24px;
  z-index: 2147483647;
  width: 340px;
  display: flex;
  flex-direction: column;
  background: #1e1f22;
  color: #dbdee1;
  border: 1px solid #2b2d31;
  border-radius: 10px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, .5);
  font-size: 13px;
}
.panel[hidden] { display: none !important; }
.panel[data-collapsed="true"] .body,
.panel[data-collapsed="true"] .footer,
.panel[data-collapsed="true"] .log { display: none; }

.header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  cursor: grab;
  border-bottom: 1px solid #2b2d31;
}
.header:active { cursor: grabbing; }
.title { flex: 1; font-weight: 600; font-size: 13px; }
.badge { font-size: 11px; color: #949ba4; }

.icon-button {
  background: none;
  border: none;
  color: #949ba4;
  cursor: pointer;
  font-size: 15px;
  line-height: 1;
  padding: 2px 5px;
  border-radius: 4px;
}
.icon-button:hover { background: #35373c; color: #dbdee1; }

.body { padding: 12px; display: grid; gap: 9px; max-height: 46vh; overflow-y: auto; }
.body, .log { scrollbar-width: thin; scrollbar-color: #4e5058 transparent; }
.body::-webkit-scrollbar, .log::-webkit-scrollbar { width: 8px; }
.body::-webkit-scrollbar-thumb, .log::-webkit-scrollbar-thumb {
  background: #4e5058;
  border-radius: 4px;
}
.body::-webkit-scrollbar-track, .log::-webkit-scrollbar-track { background: transparent; }
.body > *, .row > *, .field > * { min-width: 0; }
.field { display: grid; gap: 4px; }
.field label { font-size: 11px; text-transform: uppercase; letter-spacing: .02em; color: #949ba4; }
.field .hint { font-size: 10px; color: #6d6f78; }

input, select {
  background: #1e1f22;
  border: 1px solid #3f4147;
  border-radius: 5px;
  color: #dbdee1;
  padding: 6px 8px;
  font-size: 12px;
  width: 100%;
}
input:focus, select:focus { outline: none; border-color: #5865f2; }

.row { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.toggles { display: grid; gap: 6px; }
.toggle { display: flex; align-items: center; gap: 7px; font-size: 12px; color: #b5bac1; }
.toggle input { width: auto; }

.actions { display: flex; gap: 6px; }
button.action {
  flex: 1;
  border: none;
  border-radius: 5px;
  padding: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  color: #fff;
  background: #4e5058;
}
button.action:hover:not(:disabled) { filter: brightness(1.15); }
button.action:disabled { opacity: .4; cursor: not-allowed; }
button.action.primary { background: #5865f2; }
button.action.danger { background: #da373c; }

.link-button {
  background: none;
  border: none;
  color: #00a8fc;
  font-size: 11px;
  cursor: pointer;
  padding: 0;
  text-align: left;
  justify-self: start;
}
.link-button:hover { text-decoration: underline; }

.footer { padding: 10px 12px; border-top: 1px solid #2b2d31; display: grid; gap: 8px; }
.stats { display: flex; justify-content: space-between; font-size: 11px; color: #949ba4; }
.stats b { color: #dbdee1; font-weight: 600; }

.log {
  border-top: 1px solid #2b2d31;
  max-height: 150px;
  overflow-y: auto;
  padding: 8px 12px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 10.5px;
  line-height: 1.45;
  color: #949ba4;
}
.log div { white-space: pre-wrap; word-break: break-word; }
.log .delete { color: #7ecb7e; }
.log .skip { color: #d9a441; }
.log .dry { color: #00a8fc; }
.log .error { color: #f2777a; }

"#;
