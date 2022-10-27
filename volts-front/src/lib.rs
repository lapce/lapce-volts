pub(crate) mod components;

use components::{navbar::Navbar, token::TokenList};
use gloo_net::http::Request;
use sycamore::{
    component,
    prelude::view,
    reactive::{create_signal, provide_context_ref, Scope},
    view::View,
    web::Html,
};
use volts_core::MeUser;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq, Eq, Default)]
pub struct AppContext {
    pub login: Option<String>,
}

#[component]
fn App<G: Html>(cx: Scope) -> View<G> {
    let ctx = create_signal(cx, AppContext::default());
    provide_context_ref(cx, ctx);

    {
        let req = Request::get("/api/v1/me").send();
        sycamore::futures::spawn_local_scoped(cx, async move {
            let resp = req.await.unwrap();
            let resp: MeUser = resp.json().await.unwrap();
            let mut new_ctx = (*ctx.get()).clone();
            new_ctx.login = Some(resp.login);
            ctx.set(new_ctx);
        });
    }

    view! { cx,
        Navbar {}
        TokenList {}
    }
}

#[wasm_bindgen(start)]
pub fn start_front() {
    console_error_panic_hook::set_once();
    sycamore::render(|cx| {
        view! { cx,
            App {}
        }
    })
}
