---
id: 7mak5c3wg275gj38mwm6wph
title: Static Node Optimization
desc: ""
updated: 1650217414998
created: 1650036024318
---

Since static nodes (nodes with no reactive attributes, properties, event listeners) don't change, we don't have to hydrate these nodes at all. This is possible thanks to node being trees, where if a parent node is removed from the DOM, all children are automatically removed.

Take, for example, the following node tree:

```rust
div()
    .child(|cx| h1().cx(cx).into_node())
    .dyn_child(o, |cx, v| h2().text(v.to_string()).to_string())
```

In the above snippet, `<div>` is static, as there are no dynamic parts. It might seam like the second child is dynamic, as it is called `dyn_child`, after all, however, this, under the hood, creates a component, which in itself, is dynamic. Therefore, `<div>` is not dynamic, but rather, creates a dynamic component.

`<h1>` is static as well, even `<h2>`. So in this example, only the `DynChild` component which is created by `dyn_child`, is dynamic. Therefore, we need to hydrate very few nodes. Let's see exactly how many.

Here is what the markup could look like:

```html
<div>
  <h1></h1>
  <template id="1-2-0o"></template>
  <h2 id="3-3-0"></h2>
  <template id="1-2-0c"></template>
</div>
```

The above snippet is close to the final DOM string that will be generated on the server. If we look, we have two additional `<template>` tags. These are to set the bounds of the start and end of the component. This serves the purpose of making it much more easy and efficient to insert and remove nodes.
The nodes that have an `id` attribute are meant for only nodes that are dynamic, and must be hydrated. Therefore, why does `<h2>` have an `id` attribute if we previously said it was static?

This is because, although the node is static, it is a direct descendent of the `DynNode` component. This component adds/removes a single child. However, since the component doesn't have an actual element child, how can we remove the component's children? There are two approaches:

1. Treat a direct descendent of any component as special and add an `id` so it can be queried, and subsecuently dropped from the DOM.
2. Perform a node range between the closing and opening component delimiters, and remove them from the DOM.

I am more partial to `1`, since querying a node with an `id` is extremely fast. Ranging, I have not profiled, but I can imagine it being much more expensive.

This aforementioned optimization is definitly valid for `DynChild`, but we need to varify this assumption, and if incorrect, adjust it for all other core components and custom components.

## Component Elegibility

- [x] `DynNode`
