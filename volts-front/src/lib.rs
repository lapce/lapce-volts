pub(crate) mod components;

use components::navbar::Navbar;
use components::token::TokenList;
use gloo_net::http::Request;
use sycamore::component;
use sycamore::prelude::view;
use sycamore::reactive::{create_signal, provide_context_ref};
use sycamore::{reactive::Scope, view::View, web::Html};
use volts_core::MeUser;
use yew::{function_component, html, use_state, ContextProvider};

#[derive(Clone, PartialEq, Eq, Default)]
pub struct AppContext {
    pub login: Option<String>,
}

#[function_component(App)]
fn app() -> Html {
    let ctx = use_state(|| AppContext { login: None });
    let initiated = use_state(|| false);
    if !(*initiated) {
        initiated.set(true);
        let ctx = ctx.clone();
        let req = Request::get("/api/v1/me").send();
        wasm_bindgen_futures::spawn_local(async move {
            let resp = req.await.unwrap();
            let resp: MeUser = resp.json().await.unwrap();
            let mut new_ctx = (*ctx).clone();
            new_ctx.login = Some(resp.login);
            ctx.set(new_ctx);
        });
    }

    html! {
        // <ContextProvider<AppContext> context={(*ctx).clone()}>
        //     // <Navbar />
        //     // <TokenList />
        // </ContextProvider<AppContext>>
    }
}

#[component]
fn App<G: Html>(cx: Scope) -> View<G> {
    let ctx = create_signal(cx, AppContext::default());
    provide_context_ref(cx, ctx);

    web_sys::console::log_1(&"app run".into());

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

pub fn start_front() {
    console_error_panic_hook::set_once();
    // yew::start_app::<App>();
    sycamore::render(|cx| {
        view! { cx,
            App {}
        }
    })
}
