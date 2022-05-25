---
id: zdxhdv2u8jh9vlmc0giqma1
title: Diffing
desc: ""
updated: 1651506592655
created: 1651506110219
---

Diffing is usually performed by frameworks with a virtual DOM in order to compare
the DOM between different versions, to only update specific nodes which have changed.
However, since we are a reactive framework, this diffing is not needed at all.

We do, however, have to take special considerations with hot reload, that we wouldn't
otherwise need to.

Take the following snippet

```html
<!-- Fragment 1 -->
<div>
  <h1>Hello</h1>
</div>

<!-- Fragment 2 -->

<div>
  <h4>Hello</h4>
</div>
```

In the above two fragments, fragment 1 corresponds with the initial state of the
DOM, and fragment 2 corresponds to the desired state after an HMR. How do we get
the states back in sync?

This question is the largest primary difference between HMR and SSR, in our context.

This will be explored in the future.
