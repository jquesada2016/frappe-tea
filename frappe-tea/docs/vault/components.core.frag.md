---
id: 89y2rq2c89dgoj939kj5qux
title: Frag
desc: ""
updated: 1651505678462
created: 1650036282162
---

The `Frag` component is meant to be a transparent way of creating
children that do not have any markup.

Since components typically take a single `NodeTree`, this component would allow
passing multiple children through a single `NodeTree`.

This component is static.

## Optimisations

### Static Node Optimization

Since this is a static node, it is fully compatible with this optimization.
