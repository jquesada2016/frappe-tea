---
id: e1y2u8ocseer3lccwwlbq3n
title: SSR
desc: ""
updated: 1650216480299
created: 1649964930486
---

SSR (Server-Side Rendering) is the act of rendering the application's state into a string which will be sent to the client to render locally.

In the case of FT, the following code will be rendered into the subsecuent HTML fragment.

```rust
dov()
    .child(|cx| h1().cx(cx).text("Hello, SSR!").into_node())
```

```html
<div>
  <h1>Hello, SSR!</h1>
</div>
```

It is important for the application to be resumable, not replayable, therefore, all necessary data required to reconstruct the state of the app, must be (de)serializable.

Nodes must be able to be converted to strings, and back again. This reverse process is called **hydration**.

Going from a node tree to string is straightforward, just serialize it into a DOM string, i.e.:

```html
<tag attr="val"> etc... </tag>
```

However, going the other way around, in hydration, is another story. There are many different strategies for achieving this, but there are many optimizations we can perform to speed this costly step up significantly, thanks to the reactive nature of the framework, and other such considerations.

For example, one of our goals is to optimize time-to-interactive. this means that we can prioritize hydrating nodes that have event handlers, and let the rest be mounted asynchronously.

## Optimizations

- [[ssr.static-node-optimization]]
- [[ssr.time-to-interactive]]
