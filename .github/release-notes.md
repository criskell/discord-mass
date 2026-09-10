Packaged build of the extension.

On Firefox, open `about:debugging`, click Load Temporary Add-on and pick this zip directly.
You do not need to unpack it. If the file dialog hides `.zip` files, rename it to `.xpi` —
same format, and the picker will show it.

On Chrome, unpack the zip first, then open `chrome://extensions`, turn on developer mode
and load the resulting folder.

The package is unsigned, so Firefox forgets it when the browser restarts.
