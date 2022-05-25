---
id: w453x553097gywkajb9xns2
title: HMR
desc: ""
updated: 1649966047466
created: 1649964941193
---

HMR (Hot-Module Reload) is the act of updating a part of your app, and have it update dynamically, instead of refreshing the page each and every time.

Since `wasm` is statically built, any change, no matter how small, will most likely result in a completely different binary. Therefore, the entire `*.wasm` file has to get re-sent to the browser, without specialized tools, which I am currently unaware of. The inevitably means that true HMR, at the moment, to the best of my knowledge, is not possible.

We can, however, emulate HMR in the FT. This is, every time there's a change, we rebuild the `*.wasm` file, resend it to the browser, and have the app resume from the state right before the code change took place. As long as we support SSR, HMR should be relatively trivial to implement.

The following will detail specific implementation details regarding HMR that deviates from the SSR implementation.

SSR implementation can be found [[here|ssr]].
