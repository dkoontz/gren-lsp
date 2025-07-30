#!/usr/bin/env fish

gren make src/Main.gren --optimize --output=app.js
scp app.js index.html root@hub.nojs.de:/var/www/dedris.nojs.de/
