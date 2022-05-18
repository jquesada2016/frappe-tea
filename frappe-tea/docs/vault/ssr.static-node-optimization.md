---
id: 7mak5c3wg275gj38mwm6wph
title: Static Node Optimization
desc: ""
updated: 1651619276167
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

## Implementation Considerations

Given the fact that we would like to have nodes not be queried from the DOM unless they are dynamic, how to we differentiate a dynamic node from a static one? If any dynamic property, except [[components.core.dyn-child]] (`.dyn_child()`) or [[components.core.dyn-text] (`.dyn_text()`)] is used, then the node will be considered dynamic.
This would also mean that it would not be possible to request a reference to a node at runtime for static nodes. We could encode this explicitly in a typestate, or we could do it implicitly with a marker.

The reason we would potentially want to retrieve a reference to a node, is so that we can perform arbitrary actions on the node, such as focusing the element, etc.

```rust
div(cx) // HtmlElement<Static, Msg>
  .dyn_class(/* ... */) // HtmlElement<Dynamic, Msg>
```

What I am ultimately trying to accomplish with the above typestate between static and dynamic nodes, is to avoid users from applying cfg's that might cause subtle bugs and incompatibility between SSR and client code. If the two disagree, bugs are bound to arise. Take the following, for example.

```rust
fn my_view() -> impl IntoNode<Msg> {
  let d = div();

  let d = div.text("hello"); // static

  // Only run the following code on the client
  #[cfg(target_arch = "wasm32")]
  let d = div.dyn_class(/* ... */); // dynamic

  d
}
```

The above snippet is problematic, because as far as SSR is concerned, the node is static, because there are no dynamic elements to the node. However, this will cause the node to not be found when queried, because the server did not generate any id for the element.

There might be a couple potential fixes to the problem.

- We can default to always generating id's, and let the client decide if it cares for the node being static or dynamic, and proceed as usual.
-
