---
id: ga19jxnei222bvvyx3q7le8
title: Time to Interactive
desc: ""
updated: 1650217105044
created: 1650215825915
---

Time-to-interactive is the measurement of time from when the client receives the raw `HTML` and when it becomes interactive to the user (buttons do stuff, tabs work, etc.). It is important for this period of time to be as short as possible, which will directly influence user experience of the app.

This optimization works by assigning priority to nodes which have event handlers, hydrating those first, and mounting subsecuent nodes later.

If a node has any event handlers, it is automatically included in this optimization.

Hydration of non-eventful nodes will be scheduled in the browser by the async runtime via promises.
