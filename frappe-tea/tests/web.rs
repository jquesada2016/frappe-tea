//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]
#![cfg(feature = "web-tests")]

extern crate wasm_bindgen_test;
use frappe_tea::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn update(model: &mut usize, msg: ()) -> Option<Box<dyn IntoCmd<()>>> {
    None
}

#[wasm_bindgen_test]
async fn single_node_renders() {
    fn view(
        _model: &usize,
    ) -> impl std::future::Future<Output = [BoxNode<()>; 1]> {
        async { [html::h3().into_node()] }
    }

    let el = Element::new("body", || (0, None), update, view).await;

    let r = el.root_node();

    assert_eq!(r, &html::Body);

    assert_eq!(r.children().len(), 1);

    assert_eq!(r.children()[0], html::H3);
}

#[wasm_bindgen_test]
async fn node_can_have_single_child() {
    fn view(
        _model: &usize,
    ) -> impl std::future::Future<Output = [BoxNode<()>; 1]> {
        async { [html::div().child(html::h1()).into_node()] }
    }

    let el = Element::new("body", || (0, None), update, view).await;

    let r = el.root_node();

    assert_eq!(r.children().len(), 1);

    assert_eq!(r.children()[0], html::Div);

    assert_eq!(r.children()[0].children().len(), 1);

    assert_eq!(r.children()[0].children()[0], html::H1);
}

#[wasm_bindgen_test]
async fn node_can_have_nested_child() {
    fn view(
        _model: &usize,
    ) -> impl std::future::Future<Output = [BoxNode<()>; 1]> {
        async {
            [html::div()
                .child(
                    html::div()
                        .child(html::h1())
                        .child(html::h2())
                        .child(html::h3()),
                )
                .child(html::h2())
                .child(html::h3())
                .into_node()]
        }
    }

    Element::new("body", || (0, None), update, view).await;

    let el = Element::new("body", || (0, None), update, view).await;

    let r = el.root_node();

    assert_eq!(r.children().len(), 1);

    assert_eq!(r.children()[0], html::Div);

    assert_eq!(r.children()[0].children().len(), 3);

    assert_eq!(r.children()[0].children()[0], html::Div);
    assert_eq!(r.children()[0].children()[1], html::H2);
    assert_eq!(r.children()[0].children()[2], html::H3);

    assert_eq!(r.children()[0].children()[0].children().len(), 3);
    assert_eq!(r.children()[0].children()[0].children()[0], html::H1);
    assert_eq!(r.children()[0].children()[0].children()[1], html::H2);
    assert_eq!(r.children()[0].children()[0].children()[2], html::H3);
}

#[wasm_bindgen_test]
fn is_browser() {
    assert!(env::is_browser());
}
