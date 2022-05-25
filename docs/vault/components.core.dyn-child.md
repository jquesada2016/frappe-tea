---
id: 1dcgk5du08cbgk7u0aimpoz
title: DynChild
desc: ''
updated: 1650218558545
created: 1650217439115
---

This component allows children to be added/removed when the observer receives an update.

This component requires:

- `observer: impl Observable`
- `children_fn: impl FnMut(observer::Item) -> Option<NodeTree<Msg>>`

If the child fn returns `None`, then the child is not rendered. If the child returns `Some(_)`, the previous node is removed, and the new one is added in it's place.

## Optimizations

### Static Node Optimization

This node works with this optimization, only requiring that it's immediate child, regardless of the type, has a queryable `id` so it can be mounted.

```rust
DynChild::new(o, |cx, _| Some(
    div()
        .cx(cx)
        .child(h1().text("Dynamic!").into_node())
));
```

```html
<template id="0-0-0o"></template>
<div>
  <h1>Dynamic!</h1>
</div>
<template id="0-0-0c"></template>
```

How do we hydrate this? `<div>` needs to be removed if the children fn ever decides it should be.
