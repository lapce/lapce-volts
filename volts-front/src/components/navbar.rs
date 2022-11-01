use gloo_net::http::Request;
use sycamore::{
    component,
    prelude::view,
    reactive::{create_effect, create_signal, use_context, Scope, Signal},
    view::View,
    web::Html,
};
use volts_core::NewSessionResponse;

use crate::AppContext;

#[component]
pub fn Navbar<G: Html>(cx: Scope) -> View<G> {
    let context = use_context::<Signal<AppContext>>(cx);
    let is_logged_in = create_signal(cx, false);
    let login = create_signal(cx, "".to_string());
    create_effect(cx, move || {
        if let Some(l) = context.get().login.as_ref() {
            login.set(l.to_string());
            is_logged_in.set(true);
        } else {
            is_logged_in.set(false);
        }
    });

    let handle_login = move |_| {
        let req = Request::get("/api/private/session").send();
        sycamore::futures::spawn_local(async move {
            let resp = req.await.unwrap();
            let resp: NewSessionResponse = resp.json().await.unwrap();

            web_sys::window()
                .unwrap()
                .location()
                .set_href(&resp.url)
                .unwrap();
        });
    };

    let handle_logout = move |_| {
        let req = Request::delete("/api/private/session");
        sycamore::futures::spawn_local(async move {
            req.send().await.unwrap();

            web_sys::window().unwrap().location().set_href("/").unwrap();
        });
    };

    view! { cx,
        div(class="relative bg-coolGray-50 overflow-hidden",
            style="background-image: linear-gradient(to right top, #4264af, #4f70ba, #5b7dc4, #688acf, #7597d9, #6ca0e0, #63a9e6, #5ab2eb, #2eb9e7, #00bfdd, #00c3cd, #10c6ba);"
        ) {
            nav(class="flex flex-wrap items-center justify-between container m-auto py-6 px-0") {
                div(class="flex items-center"){
                    a(
                        class="flex items-center",
                        href="/",
                    ) {
                        img(
                        class="h-10 mr-2",
                        src="https://raw.githubusercontent.com/lapce/lapce/master/extra/images/logo.png",
                        ) {}
                        a(class="text-blue-50") {
                            "Lapce Plugins"
                        }
                    }
                }
                div(class="flex items-center"){
                    svg(
                        class="h-4 w-4 mr-2",
                        width=512,
                        height=512,
                        viewBox="0 0 512 512",
                        fill="white",
                        xmlns="http://www.w3.org/2000/svg",
                    ) {
                        path(
                            d="M256 0C114.61 0 0 114.61 0 256c0 113.1 73.345 209.05 175.07 242.91 12.81 2.35 17.48-5.56 17.48-12.35 0-6.06-.22-22.17-.35-43.53-71.21 15.46-86.23-34.32-86.23-34.32-11.645-29.58-28.429-37.45-28.429-37.45-23.244-15.88 1.76-15.56 1.76-15.56 25.699 1.8 39.209 26.38 39.209 26.38 22.84 39.12 59.92 27.82 74.51 21.27 2.32-16.54 8.93-27.82 16.25-34.22-56.84-6.45-116.611-28.43-116.611-126.52 0-27.94 9.981-50.8 26.351-68.7-2.64-6.47-11.42-32.5 2.5-67.74 0 0 21.5-6.889 70.41 26.24 20.41-5.69 42.32-8.52 64.09-8.61 21.73.1 43.64 2.92 64.09 8.61 48.87-33.129 70.32-26.24 70.32-26.24 13.97 35.24 5.19 61.27 2.55 67.74 16.41 17.9 26.32 40.76 26.32 68.7 0 98.35-59.86 119.99-116.89 126.32 9.19 7.91 17.38 23.53 17.38 47.41 0 34.22-.31 61.83-.31 70.22 0 6.85 4.6 14.82 17.6 12.32C438.72 464.96 512 369.08 512 256 512 114.61 397.37 0 255.98 0"
                        ) {}
                    }
                    (if *is_logged_in.get() {
                        view! { cx,
                            a(
                                class="text-blue-50",
                                href="/account"
                            ) { (login.get()) }
                            button(
                                class="text-blue-50 ml-4",
                                on:click=handle_logout,
                            ) {
                                "logout"
                            }
                        }
                    } else {
                        view ! { cx,
                            button(
                                class="text-blue-50",
                                on:click=handle_login,
                            ) {
                                "login"
                            }
                        }
                    })
                }
            }
        }
    }
}
