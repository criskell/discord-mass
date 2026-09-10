# discord-mass

discord-mass deletes your old Discord messages in bulk.

Build it with `rustup target add wasm32-unknown-unknown`,
`cargo install wasm-bindgen-cli --version 0.2.128 --locked` and `./build.sh`. On Firefox, open
`about:debugging`, click Load Temporary Add-on and pick `extension/manifest.json`. On Chrome, open
`chrome://extensions`, turn on developer mode and load the `extension` folder.

Open a Discord channel, then click the discord-mass icon in the browser toolbar to show the panel.
Simulate starts switched on: it lists what it would delete and deletes nothing. Switch it off once
the list looks right.

Nothing you delete comes back. Discord's rules forbid tools like this one.
