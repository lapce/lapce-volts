use gloo_net::http::Request;
use sycamore::{
    component,
    prelude::{view, Keyed},
    reactive::{create_effect, create_selector, create_signal, use_context, Scope, Signal},
    view::View,
    web::Html,
};
use volts_core::{db::models::ApiToken, ApiTokenList, EncodeApiToken, NewTokenPayload};
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlInputElement};

use crate::AppContext;

#[component(inline_props)]
fn TokenItem<'a, G: Html>(
    cx: Scope<'a>,
    token: IndexedApiToken,
    tokens: &'a Signal<Vec<IndexedApiToken>>,
) -> View<G> {
    let revoking = create_signal(cx, false);
    let handle_revoke_token = move |_| {
        let req = Request::delete(&format!("/api/v1/me/tokens/{}", token.token.id));
        sycamore::futures::spawn_local_scoped(cx, async move {
            req.send().await.unwrap();

            let mut new_tokens = (*tokens.get()).clone();
            if let Some(i) = new_tokens.iter().position(|t| t.token.id == token.token.id) {
                new_tokens.remove(i);
            }
            tokens.set(new_tokens);
        });
        revoking.set(true);
    };
    view! { cx,
        li(
            class=(
                if token.last {
                    "p-5"
                } else {
                    "p-5 border-b"
                }
            ),
        ) {
            div(class="flex justify-between items-center") {
                p {
                    (token.token.name)
                }
                (if *create_selector(cx, || *revoking.get()).get() {
                    view! {cx,
                        p(class="rounded-md p-2 border shadow") {"Revoking"}
                    }
                } else {
                    view! {cx,
                        button(
                            class="rounded-md p-2 border shadow",
                            on:click=handle_revoke_token,
                        ) {
                            "Revoke"
                        }
                    }
                })
            }
            (if let Some(text) = token.plaintext.clone() {
                view!{cx,
                    p(class="text-lg") {
                        "Make sure to copy your API token now. You won’t be able to see it again!"
                    }
                    p(
                        class="text-lg mt-2 p-4 rounded-md bg-gray-500 text-blue-50"
                    ) {
                        (text)
                    }
                }
            } else {
                view!{cx,}
            })
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
struct IndexedApiToken {
    token: ApiToken,
    last: bool,
    plaintext: Option<String>,
}

fn get_api_tokens<'a>(cx: Scope<'a>, api_tokens: &'a Signal<Vec<IndexedApiToken>>) {
    let req = Request::get("/api/v1/me/tokens").send();
    sycamore::futures::spawn_local_scoped(cx, async move {
        let resp = req.await.unwrap();
        let tokens: ApiTokenList = resp.json().await.unwrap();
        let len = tokens.api_tokens.len();
        let tokens = tokens
            .api_tokens
            .into_iter()
            .enumerate()
            .map(|(i, token)| IndexedApiToken {
                token,
                last: i + 1 == len,
                plaintext: None,
            })
            .collect();
        api_tokens.set(tokens);
    });
}

#[component]
pub fn TokenList<G: Html>(cx: Scope) -> View<G> {
    let ctx = use_context::<Signal<AppContext>>(cx);
    let creating = create_signal(cx, false);

    let new_token_name = create_signal(cx, None);
    let handle_new_token = move |_| {
        new_token_name.set(Some("".to_string()));
        creating.set(false);
    };

    let tokens = create_signal(cx, Vec::new());
    get_api_tokens(cx, tokens);

    let is_logged_in = create_signal(cx, false);

    create_effect(cx, move || {
        if ctx.get().login.is_some() {
            is_logged_in.set(true);
            get_api_tokens(cx, tokens);
        } else {
            is_logged_in.set(false);
        }
    });

    let is_new_token_show = create_selector(cx, || new_token_name.get().is_some());

    let handle_input = move |event: Event| {
        let target: HtmlInputElement = event.target().unwrap().unchecked_into();
        new_token_name.set(Some(target.value()));
    };

    let handle_create_token = move |_| {
        if let Some(name) = new_token_name.get().as_ref() {
            let req = Request::post("/api/v1/me/tokens")
                .json(&NewTokenPayload {
                    name: name.to_string(),
                })
                .unwrap();
            sycamore::futures::spawn_local_scoped(cx, async move {
                let resp: EncodeApiToken = req.send().await.unwrap().json().await.unwrap();
                let mut new_tokens = vec![IndexedApiToken {
                    token: resp.token,
                    plaintext: Some(resp.plaintext),
                    last: false,
                }];
                new_tokens.extend((*tokens.get()).clone().into_iter());
                tokens.set(new_tokens);
                creating.set(false);
                new_token_name.set(None);
            });
        }
        creating.set(true);
    };

    view! { cx,
        (if !*is_logged_in.get() {
            view! {cx, }
        } else {
            view! {cx,
                div(class="container m-auto") {
                    div(class="mt-5") {
                        h1(class="flex justify-between") {
                            span { "API Tokens" }
                            button(
                                class=(
                                    if *is_new_token_show.get() {
                                        "p-2 text-center text-blue-50 bg-blue-500 rounded-md shadow disabled:bg-gray-500"
                                    } else {
                                        "p-2 text-center text-blue-50 bg-blue-500 rounded-md shadow"
                                    }
                                ),
                                disabled=*is_new_token_show.get(),
                                on:click=handle_new_token,
                            ) {
                                "New Token"
                            }
                        }
                        p(class="py-1") {
                            "You can use the API tokens generated on this page to manage your plugins."
                        }
                        p(class="py-1") {
                            "They are stored in hashed form, so you can only download keys when you first create them."
                        }
                    }
                    (if *is_new_token_show.get() {
                        view! { cx,
                            div(class="flex mt-5") {
                                input(
                                    class="p-2 border rounded-md w-full",
                                    placeholder="New token name",
                                    disabled=*creating.get(),
                                    prop:value=(*new_token_name.get()).clone().unwrap_or_else(|| "".to_string()),
                                    on:input=handle_input,
                                ) {}
                                button(
                                    class=(
                                        if *creating.get() {
                                            "ml-6 p-2 w-20 text-blue-50 bg-blue-500 rounded-md shadow disabled:bg-gray-500"
                                        } else {
                                            "ml-6 p-2 w-20 text-blue-50 bg-blue-500 rounded-md shadow"
                                        }
                                    ),
                                    on:click=handle_create_token,
                                    disabled=*creating.get(),
                                ) {
                                    (if *creating.get() {
                                        "Creating"
                                    } else {
                                        "Create"
                                    })
                                }
                            }
                        }
                    } else {
                        view! { cx,
                        }
                    })
                    ul(class="my-5 border rounded-md") {
                        Keyed(
                            iterable=tokens,
                            view=move |cx, token| view! {cx,
                                TokenItem(token=token, tokens=tokens)
                            },
                            key=|token| token.token.id,
                        )
                    }
                }
            }
        })
    }
}
