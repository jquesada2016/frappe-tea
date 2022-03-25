use frappe_tea::prelude::*;
use futures::stream::once;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn simple() {
    let if_true: BoxNode<()> = If::new(once(async { true }), || async {
        [html::h1().into_node().await]
    })
    .into_node()
    .await;

    assert_eq!(if_true.children().len(), 1);
    assert_eq!(if_true.children()[0], html::H1);

    let if_false: BoxNode<()> = If::new(once(async { false }), || async {
        [html::h1().into_node().await]
    })
    .into_node()
    .await;

    assert!(if_false.children().is_empty());
}
